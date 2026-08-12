---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-03
agent: codex
model: default
latest_version: "0.49.0"
homepage: https://geminicli.com/
repo: https://github.com/google-gemini/gemini-cli
docs: https://geminicli.com/docs/
cli_docs: https://geminicli.com/docs/cli/cli-reference/
binaries:
  - os: macos
    binary: gemini
    alt_binaries: []
    notes: "Primary npm/Homebrew/MacPorts executable. Local macOS inspection resolved `gemini` to an nvm global npm shim at `/Users/ken/.nvm/versions/node/v22.20.0/bin/gemini`."
  - os: linux
    binary: gemini
    alt_binaries: []
    notes: "Primary npm/Homebrew/Linuxbrew executable used by official examples."
  - os: windows
    binary: gemini
    alt_binaries: ["gemini.cmd", "gemini.ps1"]
    notes: "Official examples invoke `gemini`; npm global installs normally expose Windows command shims such as `.cmd` and PowerShell shims."
install_methods:
  - os: macos
    method: npm
    command: "npm install -g @google/gemini-cli"
    notes: "Official stable global install. Requires Node.js 20.0.0+."
  - os: linux
    method: npm
    command: "npm install -g @google/gemini-cli"
    notes: "Official stable global install. Requires Node.js 20.0.0+."
  - os: windows
    method: npm
    command: "npm install -g @google/gemini-cli"
    notes: "Official stable global install. Requires Node.js 20.0.0+ and PowerShell is supported."
  - os: macos
    method: npm
    command: "npx @google/gemini-cli"
    notes: "Official no-permanent-install execution path."
  - os: linux
    method: npm
    command: "npx @google/gemini-cli"
    notes: "Official no-permanent-install execution path."
  - os: windows
    method: npm
    command: "npx @google/gemini-cli"
    notes: "Official no-permanent-install execution path."
  - os: macos
    method: brew
    command: "brew install gemini-cli"
    notes: "Official Homebrew install."
  - os: linux
    method: brew
    command: "brew install gemini-cli"
    notes: "Official Homebrew/Linuxbrew install."
  - os: macos
    method: package_manager
    command: "sudo port install gemini-cli"
    notes: "Official MacPorts install."
  - os: macos
    method: other
    command: "conda create -y -n gemini_env -c conda-forge nodejs && conda activate gemini_env && npm install -g @google/gemini-cli"
    notes: "Official Anaconda path for restricted environments; Gemini CLI is still installed via npm inside the conda environment."
  - os: linux
    method: other
    command: "conda create -y -n gemini_env -c conda-forge nodejs && conda activate gemini_env && npm install -g @google/gemini-cli"
    notes: "Official Anaconda path for restricted environments; Gemini CLI is still installed via npm inside the conda environment."
  - os: windows
    method: other
    command: "conda create -y -n gemini_env -c conda-forge nodejs && conda activate gemini_env && npm install -g @google/gemini-cli"
    notes: "Official Anaconda path for restricted environments; Gemini CLI is still installed via npm inside the conda environment."
  - os: macos
    method: other
    command: "docker run --rm -it us-docker.pkg.dev/gemini-code-dev/gemini-cli/sandbox:<version>"
    notes: "Official container execution path. Installation docs show a versioned sandbox image, not a stable latest image."
  - os: linux
    method: other
    command: "docker run --rm -it us-docker.pkg.dev/gemini-code-dev/gemini-cli/sandbox:<version>"
    notes: "Official container execution path. Installation docs show a versioned sandbox image, not a stable latest image."
  - os: windows
    method: other
    command: "docker run --rm -it us-docker.pkg.dev/gemini-code-dev/gemini-cli/sandbox:<version>"
    notes: "Official container execution path when Docker/Podman is available. Windows shell quoting may need PowerShell adaptation."
  - os: macos
    method: source
    command: "npm run start"
    notes: "Official source-tree development command from the repository root; `npm run start:prod` is the production-mode source run."
  - os: linux
    method: source
    command: "npm run start"
    notes: "Official source-tree development command from the repository root; `npm run start:prod` is the production-mode source run."
  - os: windows
    method: source
    command: "npm run start"
    notes: "Official source-tree development command from the repository root; `npm run start:prod` is the production-mode source run."
subcommands:
  - name: default
    description: "Launches Gemini CLI. Without `-p/--prompt`, a positional query defaults to interactive mode in a TTY."
    non_interactive: true
    notes: "Automation entry point when invoked with `-p/--prompt` or in a non-TTY. Interactive REPL otherwise."
  - name: mcp
    description: "Manages configured MCP servers: add, remove, list, enable, and disable."
    non_interactive: false
    notes: "Management command. Local 0.46.0 probes timed out in this worktree after emitting a ripgrep fallback warning, so wrappers should bound calls."
  - name: extensions
    description: "Manages Gemini CLI extensions."
    non_interactive: false
    notes: "Alias: `extension`. Installed 0.46.0 source exposes install, uninstall, list, update, enable, disable, link, new, validate, and config."
  - name: skills
    description: "Manages agent skills."
    non_interactive: false
    notes: "Alias: `skill`. Installed 0.46.0 source exposes list, enable, disable, install, link, and uninstall."
  - name: hooks
    description: "Manages Gemini CLI hooks."
    non_interactive: false
    notes: "Alias: `hook`. Installed 0.46.0 source exposes `hooks migrate`, including `--from-claude`."
  - name: gemma
    description: "Manages local Gemma LiteRT model routing."
    non_interactive: false
    notes: "Installed 0.46.0 source exposes setup, stop, logs, and additional local manager paths; help/list probes were not reliable in this worktree."
  - name: update
    description: "Updates Gemini CLI to the latest version."
    non_interactive: false
    notes: "Documented in the official CLI cheatsheet, but not shown by the installed 0.46.0 top-level help."
cli_switches:
  - flag: --debug
    value: ""
    scope: ["default", "diagnostics"]
    default: "false"
    description: "Runs the CLI in debug mode."
    example: "gemini -d"
    notes: "Alias: `-d`. Official docs describe verbose logging; local source says debug console opens with F12."
  - flag: --version
    value: ""
    scope: ["global", "metadata"]
    default: "false"
    description: "Prints the installed CLI version and exits."
    example: "gemini --version"
    notes: "Alias: `-v`. Local inspection returned `0.46.0`; npm latest and GitHub latest release were `0.49.0`."
  - flag: --help
    value: ""
    scope: ["global", "help"]
    default: "false"
    description: "Prints help."
    example: "gemini --help"
    notes: "Alias: `-h`. Local top-level help listed command groups but omitted most option rows unless sourced from the installed bundle."
  - flag: --model
    value: "<MODEL>"
    scope: ["default", "model_selection"]
    default: "auto"
    description: "Selects the model or alias for the session."
    example: "gemini -m gemini-2.5-flash -p \"summarize README.md\""
    notes: "Alias: `-m`. Model catalog and endpoint behavior belong to model-config research."
  - flag: --prompt
    value: "<PROMPT>"
    scope: ["default", "non_interactive"]
    default: ""
    description: "Runs headless mode with prompt text; appends stdin input when stdin is provided."
    example: "gemini -p \"summarize README.md\""
    notes: "Alias: `-p`. Official docs call this the non-interactive automation entry point."
  - flag: --prompt-interactive
    value: "<PROMPT>"
    scope: ["default", "interactive"]
    default: ""
    description: "Executes an initial prompt and continues in interactive mode."
    example: "gemini -i \"review this project\""
    notes: "Alias: `-i`. Mutually exclusive with `--prompt`."
  - flag: --worktree
    value: "[NAME]"
    scope: ["default", "workspace"]
    default: ""
    description: "Starts Gemini in a new git worktree, generating a name when no value is provided."
    example: "gemini -w fix-login"
    notes: "Alias: `-w`. Requires `experimental.worktrees: true` in settings."
  - flag: --sandbox
    value: ""
    scope: ["default", "execution"]
    default: "false"
    description: "Runs tool execution inside a sandbox."
    example: "gemini --sandbox -y -p \"your prompt here\""
    notes: "Alias: `-s`. Can also be influenced by sandbox settings and environment variables."
  - flag: --skip-trust
    value: ""
    scope: ["default", "workspace_trust"]
    default: "false"
    description: "Trusts the current workspace for the current session and bypasses the folder trust prompt."
    example: "gemini --skip-trust -p \"run checks\""
    notes: "Official trust docs recommend it for headless runs, but local 0.46.0 rejected `--skip-trust` when placed before the default command in probes."
  - flag: --approval-mode
    value: "<default|auto_edit|yolo|plan>"
    scope: ["default", "tool_approval"]
    default: "default"
    description: "Sets tool approval behavior."
    example: "gemini --approval-mode auto_edit -p \"fix lint\""
    notes: "Mutually exclusive with `--yolo`; `plan` is read-only planning mode."
  - flag: --yolo
    value: ""
    scope: ["default", "tool_approval"]
    default: "false"
    description: "Deprecated shortcut that auto-approves all actions."
    example: "gemini -y -p \"fix lint\""
    notes: "Alias: `-y`. Docs recommend `--approval-mode=yolo` instead."
  - flag: --policy
    value: "<PATH_OR_DIR,...>"
    scope: ["default", "policy"]
    default: ""
    description: "Loads additional policy files or directories."
    example: "gemini --policy ./policy.toml -p \"inspect changes\""
    notes: "Installed 0.46.0 source accepts repeated or comma-separated values."
  - flag: --admin-policy
    value: "<PATH_OR_DIR,...>"
    scope: ["default", "policy"]
    default: ""
    description: "Loads additional admin policy files or directories."
    example: "gemini --admin-policy /etc/gemini-cli/policy.toml"
    notes: "Installed 0.46.0 source accepts repeated or comma-separated values."
  - flag: --acp
    value: ""
    scope: ["default", "protocol"]
    default: "false"
    description: "Starts the agent in ACP mode."
    example: "gemini --acp"
    notes: "Present in installed 0.46.0 source but omitted from the public cheatsheet row that still lists `--experimental-acp`."
  - flag: --experimental-acp
    value: ""
    scope: ["default", "protocol"]
    default: "false"
    description: "Deprecated experimental ACP mode flag."
    example: "gemini --experimental-acp"
    notes: "Installed source says to use `--acp` instead; official cheatsheet still lists this flag."
  - flag: --experimental-zed-integration
    value: ""
    scope: ["default", "ide"]
    default: "false"
    description: "Runs in Zed editor integration mode."
    example: "gemini --experimental-zed-integration"
    notes: "Listed by the official cheatsheet; not observed in the installed 0.46.0 option builder."
  - flag: --allowed-mcp-server-names
    value: "<NAME,...>"
    scope: ["default", "mcp"]
    default: ""
    description: "Restricts allowed MCP server names for the session."
    example: "gemini --allowed-mcp-server-names github,filesystem -p \"list tools\""
    notes: "Installed source accepts repeated or comma-separated values."
  - flag: --allowed-tools
    value: "<TOOL,...>"
    scope: ["default", "tool_approval"]
    default: ""
    description: "Deprecated list of tools allowed to run without confirmation."
    example: "gemini --allowed-tools \"ShellTool(git status)\" -p \"inspect\""
    notes: "Official docs and installed source direct users to the Policy Engine instead."
  - flag: --extensions
    value: "<NAME,...>"
    scope: ["default", "extensions"]
    default: "all extensions enabled"
    description: "Selects extensions to load for the session."
    example: "gemini -e none -p \"summarize\""
    notes: "Alias: `-e`; docs use `none` to disable all extensions."
  - flag: --list-extensions
    value: ""
    scope: ["default", "extensions", "introspection"]
    default: "false"
    description: "Lists available extensions and exits."
    example: "gemini --list-extensions"
    notes: "Alias: `-l`. Local probe exited 0 with empty output."
  - flag: --resume
    value: "[SESSION]"
    scope: ["default", "sessions"]
    default: ""
    description: "Resumes a previous session by latest, index, or ID."
    example: "gemini -r latest \"continue the fix\""
    notes: "Alias: `-r`. Mutually exclusive with `--session-id` and `--session-file`."
  - flag: --session-file
    value: "<PATH>"
    scope: ["default", "sessions"]
    default: ""
    description: "Loads a session from a JSON file."
    example: "gemini --session-file ./session.json"
    notes: "Observed in installed 0.46.0 source; absent from the public cheatsheet."
  - flag: --session-id
    value: "<ID>"
    scope: ["default", "sessions"]
    default: ""
    description: "Starts a new session with a manually provided session ID."
    example: "gemini --session-id run_123 -p \"start\""
    notes: "Installed source accepts alphanumeric characters, dashes, and underscores only."
  - flag: --list-sessions
    value: ""
    scope: ["default", "sessions", "introspection"]
    default: "false"
    description: "Lists available sessions for the current project and exits."
    example: "gemini --list-sessions"
    notes: "Local probe returned text `No previous sessions found for this project.` with exit 0."
  - flag: --delete-session
    value: "<INDEX_OR_ID>"
    scope: ["default", "sessions"]
    default: ""
    description: "Deletes a session by index number or ID."
    example: "gemini --delete-session 3"
    notes: "Use `--list-sessions` first; no JSON mode documented."
  - flag: --include-directories
    value: "<DIR,...>"
    scope: ["default", "workspace"]
    default: ""
    description: "Adds directories to workspace context."
    example: "gemini --include-directories ../lib,../docs -p \"review\""
    notes: "Official cheatsheet says comma-separated or repeated values."
  - flag: --screen-reader
    value: ""
    scope: ["default", "accessibility"]
    default: "false"
    description: "Enables screen reader mode."
    example: "gemini --screen-reader"
    notes: ""
  - flag: --output-format
    value: "<text|json|stream-json>"
    scope: ["default", "non_interactive", "output"]
    default: "text"
    description: "Selects text, single JSON, or streaming JSONL output for headless runs."
    example: "gemini -p \"summarize README.md\" -o stream-json"
    notes: "Alias: `-o`. Headless docs define JSON object and JSONL event schemas."
  - flag: --raw-output
    value: ""
    scope: ["default", "output"]
    default: "false"
    description: "Disables sanitization of model output, allowing raw ANSI escape sequences."
    example: "gemini -p \"print colored text\" --raw-output --accept-raw-output-risk"
    notes: "Observed in installed 0.46.0 source; not listed in the public cheatsheet. Security warning requires explicit risk acceptance for automation."
  - flag: --accept-raw-output-risk
    value: ""
    scope: ["default", "output"]
    default: "false"
    description: "Suppresses the warning for `--raw-output`."
    example: "gemini -p \"print raw\" --raw-output --accept-raw-output-risk"
    notes: "Observed in installed 0.46.0 source; wrapper-relevant because raw model output may contain terminal control sequences."
  - flag: --fake-responses
    value: "<PATH>"
    scope: ["default", "testing"]
    default: ""
    description: "Uses fake model responses for testing."
    example: "gemini --fake-responses ./responses.json -p \"test\""
    notes: "Hidden option in installed 0.46.0 source."
  - flag: --fake-responses-non-strict
    value: "<PATH>"
    scope: ["default", "testing"]
    default: ""
    description: "Uses fake model responses for testing in non-strict mode."
    example: "gemini --fake-responses-non-strict ./responses.json -p \"test\""
    notes: "Hidden option in installed 0.46.0 source."
  - flag: --record-responses
    value: "<PATH>"
    scope: ["default", "testing"]
    default: ""
    description: "Records model responses to a file for testing."
    example: "gemini --record-responses ./responses.jsonl -p \"test\""
    notes: "Hidden option in installed 0.46.0 source."
  - flag: GEMINI_SYSTEM_MD
    value: "<true|false|PATH>"
    scope: ["system_prompt", "env"]
    default: "unset"
    description: "Enables or points to a system prompt override file."
    example: "GEMINI_SYSTEM_MD=1 gemini"
    notes: "Existence recorded here only; replacement semantics belong to the sibling system-prompt topic."
  - flag: GEMINI_WRITE_SYSTEM_MD
    value: "<true|PATH>"
    scope: ["system_prompt", "env"]
    default: "unset"
    description: "Writes the built-in system prompt to the default or specified path."
    example: "GEMINI_WRITE_SYSTEM_MD=~/prompts/DEFAULT_SYSTEM.md gemini"
    notes: "Existence recorded here only; export semantics belong to the sibling system-prompt topic."
  - flag: --scope
    value: "<user|project>"
    scope: ["mcp add", "mcp remove"]
    default: "project"
    description: "Selects configuration scope for MCP server changes."
    example: "gemini mcp add db node db-server.js --scope user"
    notes: "Alias: `-s` for MCP commands."
  - flag: --transport
    value: "<stdio|sse|http>"
    scope: ["mcp add"]
    default: "stdio"
    description: "Selects MCP server transport."
    example: "gemini mcp add api https://example.com/mcp --transport http"
    notes: "Aliases: `-t`, `--type`."
  - flag: --env
    value: "<KEY=VALUE>"
    scope: ["mcp add"]
    default: ""
    description: "Adds environment variables to an MCP server definition."
    example: "gemini mcp add slack node server.js --env SLACK_TOKEN=xoxb-xxx"
    notes: "Alias: `-e`. Can be repeated."
  - flag: --header
    value: "<HEADER: VALUE>"
    scope: ["mcp add"]
    default: ""
    description: "Adds HTTP headers for SSE and HTTP MCP transports."
    example: "gemini mcp add secure https://api.example.com/mcp --transport http --header \"Authorization: Bearer abc123\""
    notes: "Alias: `-H`. Can be repeated."
  - flag: --timeout
    value: "<MILLISECONDS>"
    scope: ["mcp add"]
    default: ""
    description: "Sets MCP server connection timeout."
    example: "gemini mcp add server node server.js --timeout 30000"
    notes: ""
  - flag: --trust
    value: ""
    scope: ["mcp add"]
    default: "false"
    description: "Trusts an MCP server and bypasses all tool-call confirmation prompts for that server."
    example: "gemini mcp add server node server.js --trust"
    notes: "Wrapper-relevant because it persists a reduced-approval posture."
  - flag: --description
    value: "<TEXT>"
    scope: ["mcp add"]
    default: ""
    description: "Sets an MCP server description."
    example: "gemini mcp add server node server.js --description \"Local tools\""
    notes: ""
  - flag: --include-tools
    value: "<TOOL,...>"
    scope: ["mcp add"]
    default: ""
    description: "Restricts an MCP server to specific tools."
    example: "gemini mcp add github npx -y @modelcontextprotocol/server-github --include-tools list_repos,get_pr"
    notes: ""
  - flag: --exclude-tools
    value: "<TOOL,...>"
    scope: ["mcp add"]
    default: ""
    description: "Excludes specific MCP server tools."
    example: "gemini mcp add server node server.js --exclude-tools delete_repo"
    notes: ""
  - flag: --session
    value: ""
    scope: ["mcp enable", "mcp disable"]
    default: "false"
    description: "Applies MCP enable/disable state only to the current session."
    example: "gemini mcp disable github --session"
    notes: "For enable, installed source describes this as clearing a session-only disable."
  - flag: --ref
    value: "<GIT_REF>"
    scope: ["extensions install"]
    default: ""
    description: "Installs an extension from a specific git ref."
    example: "gemini extensions install https://github.com/user/my-extension --ref develop"
    notes: ""
  - flag: --auto-update
    value: ""
    scope: ["extensions install"]
    default: "false"
    description: "Enables auto-update for an installed extension."
    example: "gemini extensions install https://github.com/user/my-extension --auto-update"
    notes: ""
  - flag: --pre-release
    value: ""
    scope: ["extensions install"]
    default: "false"
    description: "Enables pre-release extension versions."
    example: "gemini extensions install https://github.com/user/my-extension --pre-release"
    notes: ""
  - flag: --consent
    value: ""
    scope: ["extensions install", "extensions link", "skills install", "skills link", "gemma setup"]
    default: "false"
    description: "Acknowledges security/installation consent and skips confirmation prompts."
    example: "gemini skills install https://github.com/user/repo.git --consent"
    notes: "Wrapper-relevant for non-interactive installation flows."
  - flag: --skip-settings
    value: ""
    scope: ["extensions install"]
    default: "false"
    description: "Skips extension install-time settings configuration."
    example: "gemini extensions install ./extension --skip-settings"
    notes: "Observed in installed 0.46.0 source."
  - flag: --all
    value: ""
    scope: ["extensions uninstall", "extensions update", "skills list"]
    default: "false"
    description: "Applies the command to all applicable items or includes built-in skills."
    example: "gemini extensions update --all"
    notes: "Meaning is command-specific."
  - flag: --output-format
    value: "<text|json>"
    scope: ["extensions list"]
    default: "text"
    description: "Selects text or JSON output for installed extension listing."
    example: "gemini extensions list --output-format json"
    notes: "Alias: `-o`. Installed 0.46.0 source exposes this, but local execution timed out before producing output."
  - flag: --scope
    value: "<user|workspace>"
    scope: ["extensions enable", "extensions disable", "extensions config", "skills disable", "skills install", "skills link", "skills uninstall"]
    default: "command-specific"
    description: "Selects user/workspace scope for extension or skill state."
    example: "gemini skills install ./skill --scope workspace"
    notes: "MCP uses `project`; skills/extensions use `workspace`."
  - flag: --path
    value: "<SUBPATH>"
    scope: ["skills install"]
    default: ""
    description: "Installs a skill from a subdirectory inside a git repository source."
    example: "gemini skills install https://github.com/user/repo.git --path skills/security"
    notes: ""
  - flag: --from-claude
    value: ""
    scope: ["hooks migrate"]
    default: "false"
    description: "Migrates hooks from Claude Code to Gemini CLI."
    example: "gemini hooks migrate --from-claude"
    notes: "Observed in installed 0.46.0 source."
  - flag: --port
    value: "<PORT>"
    scope: ["gemma setup", "gemma stop"]
    default: "unknown"
    description: "Sets the LiteRT server port."
    example: "gemini gemma setup --port 8080"
    notes: "Installed source uses a `DEFAULT_PORT` constant, but the numeric value was not proven from docs/help."
  - flag: --skip-model
    value: ""
    scope: ["gemma setup"]
    default: "false"
    description: "Skips model download during Gemma setup."
    example: "gemini gemma setup --skip-model"
    notes: ""
  - flag: --start
    value: ""
    scope: ["gemma setup"]
    default: "true"
    description: "Starts the LiteRT server after Gemma setup."
    example: "gemini gemma setup --start"
    notes: ""
  - flag: --force
    value: ""
    scope: ["gemma setup"]
    default: "false"
    description: "Re-downloads the binary and model even when already present."
    example: "gemini gemma setup --force"
    notes: ""
  - flag: --lines
    value: "<N>"
    scope: ["gemma logs"]
    default: ""
    description: "Prints the last N LiteRT server log lines and exits."
    example: "gemini gemma logs --lines 100"
    notes: "Alias: `-n`."
  - flag: --follow
    value: ""
    scope: ["gemma logs"]
    default: "true when --lines is omitted"
    description: "Follows LiteRT server logs."
    example: "gemini gemma logs --follow"
    notes: "Alias: `-f`."
config_paths:
  - os: macos
    scope: user
    path: ~/.gemini/settings.json
    format: json
    notes: "Primary user settings file. Local file existed as a symlink through this session's HOME and contained top-level keys `general`, `hooks`, `security`, `tools`, and `ui`."
  - os: linux
    scope: user
    path: ~/.gemini/settings.json
    format: json
    notes: "Primary user settings file."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\settings.json"
    format: json
    notes: "Primary user settings file."
  - os: macos
    scope: repo
    path: .gemini/settings.json
    format: json
    notes: "Project settings file under a workspace; ignored while the folder is untrusted."
  - os: linux
    scope: repo
    path: .gemini/settings.json
    format: json
    notes: "Project settings file under a workspace; ignored while the folder is untrusted."
  - os: windows
    scope: repo
    path: .gemini\\settings.json
    format: json
    notes: "Project settings file under a workspace; ignored while the folder is untrusted."
  - os: macos
    scope: system
    path: /Library/Application Support/GeminiCli/system-defaults.json
    format: json
    notes: "System defaults file; can be overridden with `GEMINI_CLI_SYSTEM_DEFAULTS_PATH`."
  - os: linux
    scope: system
    path: /etc/gemini-cli/system-defaults.json
    format: json
    notes: "System defaults file; can be overridden with `GEMINI_CLI_SYSTEM_DEFAULTS_PATH`."
  - os: windows
    scope: system
    path: C:\\ProgramData\\gemini-cli\\system-defaults.json
    format: json
    notes: "System defaults file; can be overridden with `GEMINI_CLI_SYSTEM_DEFAULTS_PATH`."
  - os: macos
    scope: system
    path: /Library/Application Support/GeminiCli/settings.json
    format: json
    notes: "System override settings file; can be overridden with `GEMINI_CLI_SYSTEM_SETTINGS_PATH`."
  - os: linux
    scope: system
    path: /etc/gemini-cli/settings.json
    format: json
    notes: "System override settings file; can be overridden with `GEMINI_CLI_SYSTEM_SETTINGS_PATH`."
  - os: windows
    scope: system
    path: C:\\ProgramData\\gemini-cli\\settings.json
    format: json
    notes: "System override settings file; can be overridden with `GEMINI_CLI_SYSTEM_SETTINGS_PATH`."
  - os: macos
    scope: user
    path: ~/.gemini/GEMINI.md
    format: text
    notes: "Global context/memory file. Local file existed and was empty."
  - os: linux
    scope: user
    path: ~/.gemini/GEMINI.md
    format: text
    notes: "Global context/memory file."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\GEMINI.md"
    format: text
    notes: "Global context/memory file."
  - os: macos
    scope: repo
    path: GEMINI.md
    format: text
    notes: "Project context/memory file discovered hierarchically; filename is configurable through `context.fileName`."
  - os: linux
    scope: repo
    path: GEMINI.md
    format: text
    notes: "Project context/memory file discovered hierarchically; filename is configurable through `context.fileName`."
  - os: windows
    scope: repo
    path: GEMINI.md
    format: text
    notes: "Project context/memory file discovered hierarchically; filename is configurable through `context.fileName`."
  - os: macos
    scope: repo
    path: .geminiignore
    format: text
    notes: "Project ignore file for Gemini file discovery when `.geminiignore` support is enabled."
  - os: linux
    scope: repo
    path: .geminiignore
    format: text
    notes: "Project ignore file for Gemini file discovery when `.geminiignore` support is enabled."
  - os: windows
    scope: repo
    path: .geminiignore
    format: text
    notes: "Project ignore file for Gemini file discovery when `.geminiignore` support is enabled."
  - os: macos
    scope: repo
    path: .env
    format: text
    notes: "Project environment file loaded by the CLI unless ignored; `DEBUG` and `DEBUG_MODE` are excluded from generic project `.env` files."
  - os: linux
    scope: repo
    path: .env
    format: text
    notes: "Project environment file loaded by the CLI unless ignored; `DEBUG` and `DEBUG_MODE` are excluded from generic project `.env` files."
  - os: windows
    scope: repo
    path: .env
    format: text
    notes: "Project environment file loaded by the CLI unless ignored; `DEBUG` and `DEBUG_MODE` are excluded from generic project `.env` files."
  - os: macos
    scope: user
    path: ~/.gemini/trustedFolders.json
    format: json
    notes: "Workspace trust state. Local file existed and contained trusted absolute paths."
  - os: linux
    scope: user
    path: ~/.gemini/trustedFolders.json
    format: json
    notes: "Workspace trust state; path can be overridden with `GEMINI_CLI_TRUSTED_FOLDERS_PATH`."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\trustedFolders.json"
    format: json
    notes: "Workspace trust state; path can be overridden with `GEMINI_CLI_TRUSTED_FOLDERS_PATH`."
  - os: macos
    scope: user
    path: ~/.gemini/oauth_creds.json
    format: json
    notes: "OAuth credential state. Local file existed; wrappers should not copy or log contents."
  - os: linux
    scope: user
    path: ~/.gemini/oauth_creds.json
    format: json
    notes: "OAuth credential state. Wrappers should not copy or log contents."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\oauth_creds.json"
    format: json
    notes: "OAuth credential state. Wrappers should not copy or log contents."
  - os: macos
    scope: user
    path: ~/.gemini/google_accounts.json
    format: json
    notes: "Google account selection state. Local file existed with `active` and `old` keys."
  - os: linux
    scope: user
    path: ~/.gemini/google_accounts.json
    format: json
    notes: "Google account selection state."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\google_accounts.json"
    format: json
    notes: "Google account selection state."
  - os: macos
    scope: user
    path: ~/.gemini/mcp-oauth-tokens-v2.json
    format: other
    notes: "MCP OAuth token state. Local file existed but did not parse as plain JSON in a redacted key probe."
  - os: linux
    scope: user
    path: ~/.gemini/mcp-oauth-tokens-v2.json
    format: other
    notes: "MCP OAuth token state."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\mcp-oauth-tokens-v2.json"
    format: other
    notes: "MCP OAuth token state."
  - os: macos
    scope: user
    path: ~/.gemini/projects.json
    format: json
    notes: "Per-project state. Local file existed with a `projects` key."
  - os: linux
    scope: user
    path: ~/.gemini/projects.json
    format: json
    notes: "Per-project state."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\projects.json"
    format: json
    notes: "Per-project state."
  - os: macos
    scope: user
    path: ~/.gemini/state.json
    format: json
    notes: "General UI/state file. Local file existed with a `tipsShown` key."
  - os: linux
    scope: user
    path: ~/.gemini/state.json
    format: json
    notes: "General UI/state file."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\state.json"
    format: json
    notes: "General UI/state file."
  - os: macos
    scope: user
    path: ~/.gemini/installation_id
    format: text
    notes: "Installation identifier. Local file existed; wrappers should not treat it as config input."
  - os: linux
    scope: user
    path: ~/.gemini/installation_id
    format: text
    notes: "Installation identifier."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\installation_id"
    format: text
    notes: "Installation identifier."
env_vars:
  - name: GEMINI_CLI_HOME
    effect: "Overrides the root directory for user-level Gemini CLI configuration and storage; the CLI creates or uses a `.gemini` folder inside it."
  - name: GEMINI_CLI_TRUST_WORKSPACE
    effect: "When set to `true`, trusts the current workspace for the current session and bypasses the folder trust check."
  - name: GEMINI_CLI_TRUSTED_FOLDERS_PATH
    effect: "Overrides the default `trustedFolders.json` state path."
  - name: GEMINI_CLI_SYSTEM_DEFAULTS_PATH
    effect: "Overrides the system defaults JSON path."
  - name: GEMINI_CLI_SYSTEM_SETTINGS_PATH
    effect: "Overrides the system settings JSON path."
  - name: GEMINI_CLI_IDE_PID
    effect: "Manually associates the CLI with an IDE process for IDE integration."
  - name: GEMINI_CLI_IDE_WORKSPACE_PATH
    effect: "Helps IDE integration identify the workspace."
  - name: GEMINI_CLI_IDE_SERVER_PORT
    effect: "Helps IDE integration identify the companion IDE server port."
  - name: GEMINI_CLI_SURFACE
    effect: "Adds a custom surface label to the User-Agent header for API traffic reporting."
  - name: GEMINI_SANDBOX
    effect: "Enables or selects sandbox execution mode."
  - name: GEMINI_SANDBOX_IMAGE
    effect: "Overrides the sandbox container image."
  - name: GEMINI_SANDBOX_PROXY_COMMAND
    effect: "Runs a proxy command alongside the macOS proxied sandbox profile."
  - name: BUILD_SANDBOX
    effect: "Builds a custom sandbox image when sandbox mode and a custom sandbox Dockerfile are used."
  - name: SANDBOX_MOUNTS
    effect: "Adds host/container mounts for container sandbox execution."
  - name: SANDBOX_FLAGS
    effect: "Passes extra flags to Docker or Podman sandbox commands."
  - name: SANDBOX_SET_UID_GID
    effect: "Controls host UID/GID mapping in container sandbox execution."
  - name: NO_COLOR
    effect: "Disables ANSI color output."
  - name: DEBUG
    effect: "Can enable debug behavior in supported paths; generic project `.env` loading excludes it by default."
  - name: DEBUG_MODE
    effect: "Can enable debug behavior in supported paths; generic project `.env` loading excludes it by default."
machine_introspection:
  - command: "gemini extensions list --output-format json"
    purpose: plugins
    machine_readable: true
    output_format: json
    useful_for_codegen: false
    notes: "Installed 0.46.0 source exposes JSON output for installed extensions; local probe timed out in this worktree before returning output."
  - command: "gemini --list-extensions"
    purpose: plugins
    machine_readable: false
    output_format: text
    useful_for_codegen: false
    notes: "Lists available extensions and exits. Local probe exited 0 with empty output."
  - command: "gemini --list-sessions"
    purpose: other
    machine_readable: false
    output_format: text
    useful_for_codegen: false
    notes: "Lists session state for the current project. Local probe returned `No previous sessions found for this project.` with exit 0."
  - command: "gemini mcp list"
    purpose: mcp
    machine_readable: false
    output_format: text
    useful_for_codegen: false
    notes: "Lists configured MCP servers. No JSON mode was documented or observed; local probe timed out and should be bounded by wrappers."
  - command: "gemini skills list --all"
    purpose: tools
    machine_readable: false
    output_format: text
    useful_for_codegen: false
    notes: "Lists discovered agent skills including built-ins. No JSON mode was documented or observed; local probe timed out and should be bounded by wrappers."
  - command: "gemini gemma status"
    purpose: doctor
    machine_readable: false
    output_format: text
    useful_for_codegen: false
    notes: "Checks local Gemma/LiteRT routing status. Local probe did not return within the timeout in this worktree."
  - command: "gemini -p <prompt> --output-format json"
    purpose: other
    machine_readable: true
    output_format: json
    useful_for_codegen: false
    notes: "Structured headless run output: response, stats, and optional error. This is model-run output, not config/provider introspection."
  - command: "gemini -p <prompt> --output-format stream-json"
    purpose: other
    machine_readable: true
    output_format: jsonl
    useful_for_codegen: false
    notes: "Structured streaming headless run output with JSONL events: init, message, tool_use, tool_result, error, and result."
wrapper_notes:
  - "Use `gemini -p/--prompt` for non-interactive runs. A positional query defaults to interactive mode in a TTY."
  - "Prefer `--output-format stream-json` for streaming wrappers and `--output-format json` for one-shot wrappers; these are run-output formats, not general management-command JSON modes."
  - "Local installed version was `0.46.0`, while npm latest and GitHub latest release were `0.49.0`; wrappers should probe version and tolerate option drift."
  - "The installed 0.46.0 top-level help omitted most option rows, but installed source and official docs expose the broader default-command flag set."
  - "In local 0.46.0 probes, long-form default-command flags such as `--prompt`, `--skip-trust`, and `--output-format` were rejected in some argv positions, while short `-p/-o` reached execution; wrappers should verify current parser behavior before relying on a normalized argv form."
  - "Subcommand help and state-listing commands in this worktree timed out after emitting `Ripgrep is not available. Falling back to GrepTool.`; any wrapper introspection should use strict timeouts."
  - "An untrusted workspace in headless mode can exit with a trust error; official docs recommend `--skip-trust` or `GEMINI_CLI_TRUST_WORKSPACE=true`, but injecting either silently changes trust posture."
  - "`--approval-mode=yolo`, `--yolo`, MCP `--trust`, and workspace trust bypasses reduce approval prompts and should require explicit user authorization."
  - "Extension and skill install/link flows may prompt for consent; command-specific `--consent` should only be used after explicit authorization."
  - "`--raw-output` can allow terminal control sequences from model output; wrappers should avoid it or pair it with their own sanitization."
  - "Set `GEMINI_CLI_HOME` to isolate wrapper config/auth state. This host's `HOME` was `/Users/ken/.claudine`, with `.gemini` entries symlinked to `/Users/ken/.gemini`."
  - "Do not log or copy credential/state files such as `oauth_creds.json`, `google_accounts.json`, `mcp-oauth-tokens-v2.json`, or `installation_id`."
  - "Official headless exit codes are 0 success, 1 general/API failure, 42 input error, and 53 turn limit exceeded; local trust failure returned exit 55."
  - "The official site banner says unpaid tier and Google One users will be replaced by Antigravity CLI on June 18; wrappers should expect auth/product behavior changes for those user classes."
changes:
  - "Verified npm latest and GitHub latest release as 0.49.0, while the local installed binary remains 0.46.0."
  - "Added evidence that local 0.46.0 top-level help omits most options and that subcommand help/listing probes can hang unless bounded."
  - "Recorded local trust behavior: untrusted worktree probes exited 55 and docs recommend `--skip-trust` or `GEMINI_CLI_TRUST_WORKSPACE=true`."
  - "Added installed-source-only flags including `--acp`, `--policy`, `--admin-policy`, `--session-file`, `--session-id`, `--raw-output`, and hidden fake/record response flags."
  - "Added `extensions list --output-format json` as the one observed management command with a machine-readable mode, while noting the local probe timeout."
  - "Expanded config discovery from `~/.gemini` inspection, including auth, account, project, state, installation, MCP OAuth, and trusted-folder state files without exposing secret values."
  - "Normalized frontmatter to the current schema by replacing cross-platform `os: all` records with explicit macOS, Linux, and Windows records."
requires_claudine_update: true
reason: "Claudine's Gemini wrapper/provider metadata should account for version drift, trust failure exit 55, reliable use of `GEMINI_CLI_HOME`, stream-json/json headless output, raw-output sanitization risk, and timeout-bounded introspection; existing metadata that assumes stable help output or `os: all` schema records is insufficient."
---

# Gemini CLI Public CLI Surface

## Overview

Gemini CLI is Google's open-source terminal agent for using Gemini models from a local shell. The primary user-facing command is `gemini`. It defaults to an interactive REPL, but `-p` / `--prompt` runs a prompt in headless mode for automation.

The current upstream stable version verified on 2026-07-03 is `0.49.0`. I verified that three ways: `npm view @google/gemini-cli version` returned `0.49.0`; `npm view @google/gemini-cli dist-tags --json` showed `latest: 0.49.0`; and GitHub Releases marks `Release v0.49.0` as latest. The installed local binary is older: `gemini --version` returned `0.46.0`.

Primary official URLs:

| Purpose | URL |
| --- | --- |
| Homepage | <https://geminicli.com/> |
| Repository | <https://github.com/google-gemini/gemini-cli> |
| General docs | <https://geminicli.com/docs/> |
| CLI cheatsheet | <https://geminicli.com/docs/cli/cli-reference/> |
| Installation | <https://geminicli.com/docs/get-started/installation/> |
| Configuration reference | <https://geminicli.com/docs/reference/configuration/> |
| Headless mode | <https://geminicli.com/docs/cli/headless/> |

## Installation and Binaries

The npm package declares one binary, `gemini`, mapped to `bundle/gemini.js`. Local macOS inspection resolved it through nvm:

```text
/Users/ken/.nvm/versions/node/v22.20.0/bin/gemini
```

Official examples use `gemini` on every OS. On Windows, npm global installs normally expose command shims such as `gemini.cmd` and `gemini.ps1`; wrappers should use normal command resolution rather than hard-coding a POSIX path.

Official requirements are Node.js 20.0.0+, Bash/Zsh/PowerShell, internet access, and supported current macOS, Windows, or Ubuntu baselines.

Official install and run commands:

| OS | Method | Command |
| --- | --- | --- |
| macOS/Linux/Windows | npm | `npm install -g @google/gemini-cli` |
| macOS/Linux/Windows | npx | `npx @google/gemini-cli` |
| macOS/Linux | Homebrew | `brew install gemini-cli` |
| macOS | MacPorts | `sudo port install gemini-cli` |
| macOS/Linux/Windows | Anaconda plus npm | `conda create -y -n gemini_env -c conda-forge nodejs && conda activate gemini_env && npm install -g @google/gemini-cli` |
| macOS/Linux/Windows | Container | `docker run --rm -it us-docker.pkg.dev/gemini-code-dev/gemini-cli/sandbox:<version>` |
| macOS/Linux/Windows | Source development | `npm run start` |
| macOS/Linux/Windows | Source production mode | `npm run start:prod` |

The installation docs also show `npx https://github.com/google-gemini/gemini-cli` for running the main branch directly.

## Subcommands

Local `gemini --help` for installed 0.46.0 showed these top-level command groups:

| Command | Description | Automation/TTY posture |
| --- | --- | --- |
| `gemini [query..]` | Launches Gemini CLI. | Automation entry point only with `-p/--prompt` or non-TTY headless use; otherwise interactive. |
| `gemini mcp` | Manages MCP servers. | Management command; local probes should be timeout-bounded. |
| `gemini extensions` / `gemini extension` | Manages extensions. | Management command; install/link flows can require consent. |
| `gemini skills` / `gemini skill` | Manages agent skills. | Management command; install/link flows can require consent. |
| `gemini hooks` / `gemini hook` | Manages Gemini CLI hooks. | Management command; `hooks migrate` can modify hook config. |
| `gemini gemma` | Manages local Gemma LiteRT model routing. | Management command; setup can download/start local services and can require consent. |
| `gemini update` | Updates Gemini CLI. | Documented in the official cheatsheet but not shown by local 0.46.0 top-level help. |

Installed 0.46.0 source exposed these subcommands under the command groups:

| Group | Subcommands observed in installed source |
| --- | --- |
| `mcp` | `add`, `remove`, `list`, `enable`, `disable` |
| `extensions` | `install`, `uninstall`, `list`, `update`, `enable`, `disable`, `link`, `new`, `validate`, `config` |
| `skills` | `list`, `enable`, `disable`, `install`, `link`, `uninstall` |
| `hooks` | `migrate` |
| `gemma` | `setup`, `stop`, `logs`; the old research listed `start` and `status`, but I did not prove those from the installed source slice or reliable help output in this run |

Subcommand help and several list/status commands were not reliable locally: in this worktree, probes such as `gemini mcp -h`, `gemini mcp list`, `gemini extensions list`, `gemini skills list --all`, and `gemini gemma status` timed out after printing `Ripgrep is not available. Falling back to GrepTool.`. Treat that as wrapper evidence: any background introspection should run with a short timeout and should tolerate no result.

## CLI Switch Inventory

The frontmatter contains the complete switch inventory distilled for Claudine metadata. The most wrapper-relevant switches are:

| Scope | Switches |
| --- | --- |
| Headless execution | `-p/--prompt`, `-o/--output-format`, `--include-directories`, `--resume`, `--session-file`, `--session-id`, `--list-sessions`, `--delete-session` |
| Output safety | `--raw-output`, `--accept-raw-output-risk` |
| Approval/trust posture | `--approval-mode`, `--yolo`, `--skip-trust`, MCP `--trust`, `GEMINI_CLI_TRUST_WORKSPACE=true` |
| Execution sandbox | `--sandbox`, `GEMINI_SANDBOX`, `GEMINI_SANDBOX_IMAGE`, `SANDBOX_MOUNTS`, `SANDBOX_FLAGS` |
| Policy | `--policy`, `--admin-policy`, deprecated `--allowed-tools` |
| MCP narrowing | `--allowed-mcp-server-names`, MCP add `--include-tools`, MCP add `--exclude-tools` |
| Extensions/skills | `--extensions`, `--list-extensions`, command-specific `--scope`, `--consent`, `--all`, `--ref`, `--auto-update`, `--pre-release`, `--path` |
| Protocol/IDE | `--acp`, deprecated `--experimental-acp`, documented `--experimental-zed-integration` |
| System prompt existence only | `GEMINI_SYSTEM_MD`, `GEMINI_WRITE_SYSTEM_MD` |

The system-prompt controls are intentionally not documented semantically here. This topic records only that `GEMINI_SYSTEM_MD=<true|false|PATH>` and `GEMINI_WRITE_SYSTEM_MD=<true|PATH>` exist, with examples such as `GEMINI_SYSTEM_MD=1 gemini` and `GEMINI_WRITE_SYSTEM_MD=~/prompts/DEFAULT_SYSTEM.md gemini`. Replacement-vs-append behavior, file-vs-inline behavior, and mode interactions belong to the sibling `system-prompt` research topic.

Help/docs disagreement:

- The official CLI cheatsheet lists `--experimental-acp` and `--experimental-zed-integration`, but the installed 0.46.0 option builder has `--acp` and deprecated `--experimental-acp`; I did not observe `--experimental-zed-integration` locally.
- The installed 0.46.0 source exposes `--policy`, `--admin-policy`, `--session-file`, `--session-id`, `--raw-output`, `--accept-raw-output-risk`, and hidden fake/record response flags that the public cheatsheet did not fully list.
- The local top-level help printed command groups but omitted most options. For option inventory I trusted installed source plus official docs over local help because local help was incomplete.
- Local argv probes rejected some long-form default-command flags in certain positions, including `--prompt`, `--skip-trust`, and `--output-format`; short `-p/-o` reached execution. Wrappers should probe current behavior and prefer a known-good argv order for the installed version.

## Configuration Discovery

Gemini CLI applies configuration in this precedence order: hardcoded defaults, system defaults file, user settings, project settings, system settings, environment variables, then CLI arguments.

Primary settings files:

| Scope | macOS | Linux | Windows | Format |
| --- | --- | --- | --- | --- |
| System defaults | `/Library/Application Support/GeminiCli/system-defaults.json` | `/etc/gemini-cli/system-defaults.json` | `C:\ProgramData\gemini-cli\system-defaults.json` | JSON |
| User settings | `~/.gemini/settings.json` | `~/.gemini/settings.json` | `%USERPROFILE%\.gemini\settings.json` | JSON |
| Project settings | `.gemini/settings.json` | `.gemini/settings.json` | `.gemini\settings.json` | JSON |
| System overrides | `/Library/Application Support/GeminiCli/settings.json` | `/etc/gemini-cli/settings.json` | `C:\ProgramData\gemini-cli\settings.json` | JSON |

The system paths can be redirected with `GEMINI_CLI_SYSTEM_DEFAULTS_PATH` and `GEMINI_CLI_SYSTEM_SETTINGS_PATH`.

Context and project discovery:

- `~/.gemini/GEMINI.md` is the global context/memory file.
- `GEMINI.md` files in project directories provide hierarchical project context; the filename is configurable through `context.fileName`.
- `.geminiignore` controls file discovery when Gemini ignore support is enabled.
- `.env` files can be loaded. Generic project `.env` files exclude `DEBUG` and `DEBUG_MODE` by default; `.gemini/.env` files are not subject to that exclusion.

Local host inspection:

- In this session, `HOME` was `/Users/ken/.claudine`.
- `$HOME/.gemini` existed and its entries were symlinked to `/Users/ken/.gemini`.
- Local files included `settings.json`, `GEMINI.md`, `trustedFolders.json`, `oauth_creds.json`, `google_accounts.json`, `mcp-oauth-tokens-v2.json`, `projects.json`, `state.json`, `installation_id`, `history/`, `tmp/`, and `antigravity/`.
- `settings.json` parsed as JSON with top-level keys `general`, `hooks`, `security`, `tools`, and `ui`.
- `trustedFolders.json` parsed as JSON and contained trusted absolute paths.
- `oauth_creds.json` and `google_accounts.json` existed; they are credential/account state and should never be logged by wrappers.
- `mcp-oauth-tokens-v2.json` existed but did not parse as plain JSON during a redacted key probe.

Trust side effects matter for wrappers. The trusted-folders docs say untrusted workspaces disable project settings, project `.env`, extension management, auto-acceptance, automatic memory loading, MCP server connections, and custom commands. In headless mode, an untrusted workspace can fail instead of prompting. Local probes in this worktree returned a trust error with exit 55 unless trust was bypassed by environment.

## Environment Variables

General CLI/runtime variables recorded here:

| Variable | Effect |
| --- | --- |
| `GEMINI_CLI_HOME` | Moves user-level Gemini CLI config/storage; the CLI uses a `.gemini` directory under this root. |
| `GEMINI_CLI_TRUST_WORKSPACE` | `true` trusts the current workspace for this session and bypasses the trust prompt. |
| `GEMINI_CLI_TRUSTED_FOLDERS_PATH` | Overrides the trusted-folder state file path. |
| `GEMINI_CLI_SYSTEM_DEFAULTS_PATH` | Overrides the system defaults JSON path. |
| `GEMINI_CLI_SYSTEM_SETTINGS_PATH` | Overrides the system settings JSON path. |
| `GEMINI_CLI_IDE_PID` | Associates the CLI with a specific IDE process. |
| `GEMINI_CLI_IDE_WORKSPACE_PATH` | Supplies workspace identity to IDE integration. |
| `GEMINI_CLI_IDE_SERVER_PORT` | Supplies the IDE companion server port. |
| `GEMINI_CLI_SURFACE` | Adds a custom surface label to API User-Agent traffic reporting. |
| `GEMINI_SANDBOX` | Enables/selects sandbox mode. |
| `GEMINI_SANDBOX_IMAGE` | Overrides the sandbox container image. |
| `GEMINI_SANDBOX_PROXY_COMMAND` | Runs a proxy command for the macOS proxied sandbox profile. |
| `BUILD_SANDBOX` | Builds a custom sandbox image in supported sandbox flows. |
| `SANDBOX_MOUNTS` | Adds sandbox host/container mounts. |
| `SANDBOX_FLAGS` | Passes extra flags to Docker or Podman sandbox commands. |
| `SANDBOX_SET_UID_GID` | Controls host UID/GID mapping in container sandboxes. |
| `NO_COLOR` | Disables ANSI color output. |
| `DEBUG` / `DEBUG_MODE` | Can affect debug behavior; generic project `.env` files exclude them by default. |

Authentication/model endpoint variables such as `GEMINI_API_KEY`, `GEMINI_MODEL`, `GOOGLE_API_KEY`, `GOOGLE_CLOUD_PROJECT`, and `GOOGLE_APPLICATION_CREDENTIALS` are intentionally not expanded here because they belong to model-config/auth research. System-prompt environment variables are recorded only as switch existence in this topic and deferred to the sibling `system-prompt` topic for semantics.

## Machine Introspection

Gemini CLI has only limited machine-readable provider-state introspection.

| Command | Machine-readable | Format | Usefulness |
| --- | --- | --- | --- |
| `gemini extensions list --output-format json` | Yes, by installed source | JSON | Potential extension inventory. Local probe timed out in this worktree, so wrappers need a timeout and fallback. |
| `gemini --list-extensions` | No proven structure | Text | Local probe exited 0 with empty output. Useful only as a coarse report surface. |
| `gemini --list-sessions` | No | Text | Local probe returned `No previous sessions found for this project.` Useful for resume UI, not codegen. |
| `gemini mcp list` | No proven structure | Text | Lists configured MCP servers according to source/docs; local probe timed out. |
| `gemini skills list --all` | No proven structure | Text | Lists discovered skills including built-ins; local probe timed out. |
| `gemini gemma status` | No proven structure | Text | Local Gemma/LiteRT diagnostic; local probe did not complete within timeout. |
| `gemini -p <prompt> --output-format json` | Yes | JSON | Structured model-run output, not provider-state introspection. |
| `gemini -p <prompt> --output-format stream-json` | Yes | JSONL | Structured model-run stream with `init`, `message`, `tool_use`, `tool_result`, `error`, and `result` events. |

There is no documented general `config dump`, effective config report, model catalog command, doctor command, MCP JSON listing, tool registry JSON listing, or capability report in the public surface I verified. `--help` and `--version` are useful probes but are not counted as machine introspection here because they do not expose machine-usable provider state.

## Wrapper Notes

Use `-p/--prompt` for non-interactive runs. A bare positional query is not a safe automation interface in a TTY because the documented behavior is interactive continuation. For structured output, use `--output-format json` for a single response object or `--output-format stream-json` for JSONL event streaming.

Do not silently add `--approval-mode=yolo`, `--yolo`, MCP `--trust`, `--skip-trust`, or `GEMINI_CLI_TRUST_WORKSPACE=true`. They change the user's approval or trust posture. If a headless run fails because the workspace is untrusted, surface that state or require explicit policy before bypassing it.

Use `GEMINI_CLI_HOME` for wrapper-isolated state. On this host, `HOME` is already redirected to `/Users/ken/.claudine`, and `$HOME/.gemini` contains symlinks into `/Users/ken/.gemini`; wrappers should not assume `~/.gemini` is the OS account home path unless they control `HOME` and `GEMINI_CLI_HOME`.

Treat local management/introspection commands as potentially slow or interactive. In this run, several read-only management commands timed out and emitted a ripgrep fallback warning. Bound them with timeouts and avoid using their output for code generation unless a JSON mode is proven for the installed version.

Be careful with shell quoting. MCP `add` accepts unknown options as server args and supports a `--` separator; wrappers should pass argv arrays instead of shell strings, especially for `--env`, `--header`, and server command arguments.

Avoid `--raw-output` unless the wrapper owns terminal sanitization. The installed source explicitly describes raw output as allowing ANSI escape sequences and requiring risk acceptance.

Do not print credential/state files. Local config includes OAuth credentials, Google account selection, MCP OAuth token state, trusted folder state, project state, and installation IDs.

Official headless exit codes are 0 success, 1 general/API failure, 42 input error, and 53 turn limit exceeded. Local untrusted-workspace failure returned exit 55, so wrappers should map that separately from generic provider failure.

## Changelog

- 2026-07-03: Verified npm latest and GitHub latest release as `0.49.0`; local installed binary remains `0.46.0`.
- 2026-07-03: Added local evidence that top-level help omits most default-command flags and that several subcommand help/listing probes can hang without timeouts.
- 2026-07-03: Added local trust failure behavior: untrusted headless probes returned exit 55 and official docs recommend `--skip-trust` or `GEMINI_CLI_TRUST_WORKSPACE=true`.
- 2026-07-03: Added installed-source flags not fully covered by public cheatsheet, including `--acp`, `--policy`, `--admin-policy`, `--session-file`, `--session-id`, `--raw-output`, `--accept-raw-output-risk`, and hidden fake/record response flags.
- 2026-07-03: Added `extensions list --output-format json` as a potential machine-readable extension listing surface while noting that local execution timed out.
- 2026-07-03: Expanded configuration discovery from `~/.gemini` inspection, including settings, trust, account, OAuth, MCP OAuth, project, state, and installation files.
- 2026-07-03: Reworked frontmatter to match the current schema by replacing cross-platform `os: all` records with explicit macOS, Linux, and Windows records.

## Sources

- [Gemini CLI homepage](https://geminicli.com/)
- [Gemini CLI documentation](https://geminicli.com/docs/)
- [Gemini CLI GitHub repository](https://github.com/google-gemini/gemini-cli)
- [Gemini CLI installation, execution, and releases](https://geminicli.com/docs/get-started/installation/)
- [Gemini CLI cheatsheet](https://geminicli.com/docs/cli/cli-reference/)
- [Gemini CLI configuration reference](https://geminicli.com/docs/reference/configuration/)
- [Gemini CLI headless mode reference](https://geminicli.com/docs/cli/headless/)
- [Gemini CLI trusted folders](https://geminicli.com/docs/cli/trusted-folders/)
- [Gemini CLI system prompt override](https://geminicli.com/docs/cli/system-prompt/)
- [Gemini CLI GitHub releases](https://github.com/google-gemini/gemini-cli/releases)
- [@google/gemini-cli npm package](https://www.npmjs.com/package/@google/gemini-cli)
- Local command, 2026-07-03: `which gemini` and `command -v gemini` resolved `/Users/ken/.nvm/versions/node/v22.20.0/bin/gemini`.
- Local command, 2026-07-03: `gemini --version` returned `0.46.0`.
- Local command, 2026-07-03: `npm view @google/gemini-cli version dist-tags bin engines repository homepage --json --color=false` returned latest `0.49.0`, preview `0.50.0-preview.1`, nightly `0.51.0-nightly.20260703.gf7af4e518`, bin `gemini: bundle/gemini.js`, and Node engine `>=20`.
- Local command, 2026-07-03: `gemini --help` listed top-level command groups but omitted most option details.
- Local command, 2026-07-03: installed package source under `/Users/ken/.nvm/versions/node/v22.20.0/lib/node_modules/@google/gemini-cli` was inspected for yargs command and option definitions.
- Local command, 2026-07-03: redacted `~/.gemini` inspection listed file names, file types, sizes, and JSON top-level keys without printing secret values.
- Local negative probes, 2026-07-03: `gemini mcp -h`, `gemini extensions -h`, `gemini skills -h`, `gemini hooks -h`, `gemini gemma -h`, `gemini mcp list`, `gemini extensions list`, `gemini skills list --all`, and `gemini gemma status` did not return reliable output in this worktree under bounded timeouts.
