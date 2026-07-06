---
$schema: ./_schema.yaml
created: 2026-07-01
last_updated: 2026-07-03
agent: codex
model: default

cli_params:
  - param: approval-mode
    style: switch
    description: "Set the session approval mode. Current values are plan, default, auto-edit, auto, and yolo."
    example: "qwen -p \"review this repository\" --approval-mode plan"
    example_description: "Starts a headless read-only planning run."
  - param: yolo
    style: switch
    description: "Shortcut for YOLO approval mode. It auto-approves tool calls but does not enable a sandbox."
    example: "qwen -p \"run tests and fix failures\" --yolo"
    example_description: "Starts a headless run with all tool calls auto-approved unless other hard blocks apply."
  - param: allowed-tools
    style: switch
    description: "Comma-separated or repeated list of permission rules to auto-approve for this session; accepts the same Tool or Tool(specifier) grammar as permissions.allow."
    example: "qwen -p \"run the unit tests\" --allowed-tools \"Bash(npm test),Read\""
    example_description: "Auto-approves npm test and read-family tools for this run."
  - param: exclude-tools
    style: switch
    description: "Comma-separated or repeated list of permission rules to deny for this session; accepts tool aliases, command patterns, path rules, and MCP tool names."
    example: "qwen -p \"inspect only\" --exclude-tools \"Bash,Edit,Write,NotebookEdit,WebFetch,Agent,Skill,mcp__*\""
    example_description: "Blocks common mutation, network, subagent, skill, and MCP tool surfaces for one run."
  - param: core-tools
    style: switch
    description: "Comma-separated or repeated legacy core-tool allowlist. Only listed core tools are registered, but this is deprecated in favor of permissions.allow/deny and is ignored in safe mode."
    example: "qwen -p \"summarize the repo\" --core-tools read_file,grep_search,glob,list_directory"
    example_description: "Limits registered core tools to read/search tools for the session."
  - param: safe-mode
    style: switch
    description: "Disable customizations such as context files, hooks, extensions, skills, MCP servers, custom subagents, permission rules from settings, memory features, and sandbox settings. Explicit --approval-mode, --yolo, --allowed-tools, and --exclude-tools still apply."
    example: "qwen -p \"diagnose startup\" --safe-mode --approval-mode plan"
    example_description: "Starts a troubleshooting run with project/user customizations disabled and plan approval mode forced by CLI."
  - param: bare
    style: switch
    description: "Minimal mode that skips implicit startup auto-discovery and honors only explicit CLI inputs. It also disables settings-sourced permission rules, MCP servers, extensions, skills, memory, hooks, and sandbox settings."
    example: "qwen --bare -p \"summarize the prompt input\" --approval-mode plan"
    example_description: "Runs with minimal startup behavior and read-only plan mode."
  - param: sandbox
    style: switch
    description: "Enable sandboxing for the session. The flag is boolean in current CLI help; docs also describe --sandbox=<provider>, while QWEN_SANDBOX can force docker, podman, or sandbox-exec."
    example: "qwen -s -p \"run the test suite\""
    example_description: "Runs the headless prompt with sandboxing enabled."
  - param: sandbox-image
    style: switch
    description: "Set the Docker/Podman sandbox image for this session. The flag is deprecated in favor of tools.sandboxImage but currently has highest sandbox-image precedence."
    example: "qwen -s --sandbox-image ghcr.io/qwenlm/qwen-code:0.19.6 -p \"build\""
    example_description: "Uses a specific container image for the sandboxed run."
  - param: include-directories
    style: switch
    description: "Add extra directories to the workspace context. Can be repeated or comma-separated; docs state a maximum of five directories."
    example: "qwen -p \"inspect shared code\" --include-directories ../shared,../docs"
    example_description: "Includes adjacent directories in the workspace scope for this session."
  - param: allowed-mcp-server-names
    style: switch
    description: "Comma-separated or repeated list of MCP server names to allow for this session. When set, settings-level mcp.allowed and mcp.excluded are ignored."
    example: "qwen --allowed-mcp-server-names github,filesystem -p \"triage issue\""
    example_description: "Loads only the named MCP servers from the effective MCP configuration."
  - param: mcp-config
    style: switch
    description: "Load session-injected MCP servers from an inline JSON string or JSON file path with an mcpServers object. CLI/session-injected servers sit above settings and .mcp.json and are not gated by project MCP approval."
    example: "qwen --mcp-config ./mcp.json -p \"query the test database\""
    example_description: "Adds MCP servers for this run without editing settings.json."
  - param: extensions
    style: switch
    description: "Comma-separated or repeated extension names to use for this session. If omitted, all available extensions are used; docs describe `qwen -e none` to disable all extensions."
    example: "qwen -e none -p \"review without extension tools\""
    example_description: "Disables extension-provided tools, commands, and subagents for the run."
  - param: disabled-slash-commands
    style: switch
    description: "Comma-separated or repeated slash command names to hide and refuse. Unioned with slashCommands.disabled and QWEN_DISABLED_SLASH_COMMANDS."
    example: "qwen --disabled-slash-commands auth,mcp,extensions"
    example_description: "Removes high-risk slash commands from the interactive command surface."
  - param: max-tool-calls
    style: switch
    description: "Headless/unattended run budget for cumulative top-level tool calls. `0` means no tool calls are allowed; the first attempted tool call aborts with budget exit code 55."
    example: "qwen -p \"answer without tools\" --max-tool-calls 0"
    example_description: "Provides a hard session-scoped no-tool-execution budget, but it cannot selectively add tools back in the same run."
  - param: max-wall-time
    style: switch
    description: "Headless/unattended wall-clock budget. Accepts seconds or duration strings such as 30s, 5m, or 1h."
    example: "qwen -p \"try a bounded fix\" --max-wall-time 10m"
    example_description: "Prevents an unattended run from exceeding a wall-clock limit."
  - param: max-session-turns
    style: switch
    description: "Maximum number of session turns before exiting; useful as an unattended-run guardrail."
    example: "qwen -p \"attempt one fix\" --max-session-turns 8"
    example_description: "Caps the session turn count for a headless task."
  - param: max-subagent-depth
    style: switch
    description: "Maximum subagent nesting depth, one-based. `1` keeps subagents available but prevents nested subagents; capped at 100."
    example: "qwen -p \"investigate\" --max-subagent-depth 1"
    example_description: "Allows first-level delegation but blocks recursive subagent nesting."
  - param: output-format
    style: switch
    description: "Set headless output format to text, json, or stream-json. Stream-json can carry permission events through SDK/daemon integrations."
    example: "qwen -p \"fix lint\" --output-format stream-json"
    example_description: "Streams machine-readable events during a headless run."

env_vars:
  - name: QWEN_HOME
    effect: "Changes the global Qwen directory, including user settings, memory, skills, credentials, trusted folders, and runtime state when QWEN_RUNTIME_DIR is unset."
    effect_category: state_home_relocation
  - name: QWEN_RUNTIME_DIR
    effect: "Separates runtime output such as conversations, logs, and todos from persistent global config."
    effect_category: state_home_relocation
  - name: QWEN_CODE_SAFE_MODE
    effect: "Truthy values enable safe mode, disabling settings-sourced permission rules, MCP servers, extensions, skills, hooks, memory features, custom subagents, and sandbox settings while still honoring explicit CLI approval flags."
    effect_category: customization_lockdown
  - name: QWEN_SANDBOX
    effect: "Enables or disables sandboxing and can force docker, podman, or sandbox-exec. For sandbox enablement, docs state this overrides the CLI flag and settings."
    effect_category: sandbox_control
  - name: QWEN_SANDBOX_IMAGE
    effect: "Sets the sandbox image unless --sandbox-image is supplied; overrides tools.sandboxImage."
    effect_category: sandbox_control
  - name: SEATBELT_PROFILE
    effect: "macOS-only Seatbelt profile selector, including permissive-open, permissive-closed, permissive-proxied, restrictive-open, restrictive-closed, restrictive-proxied, and project custom profiles."
    effect_category: sandbox_control
  - name: SANDBOX_FLAGS
    effect: "Additional Docker/Podman flags for container sandboxing."
    effect_category: sandbox_control
  - name: QWEN_SANDBOX_PROXY_COMMAND
    effect: "Starts a local proxy for proxied sandbox profiles; used for network allowlist-style control."
    effect_category: sandbox_control
  - name: SANDBOX_SET_UID_GID
    effect: "Linux container sandbox UID/GID mapping control."
    effect_category: sandbox_control
  - name: QWEN_DISABLED_SLASH_COMMANDS
    effect: "Comma-separated slash-command denylist unioned with slashCommands.disabled and --disabled-slash-commands."
    effect_category: tool_surface
  - name: QWEN_CODE_SYSTEM_SETTINGS_PATH
    effect: "Overrides the system settings file path, which is the highest settings-file layer."
    effect_category: config_path_override
  - name: QWEN_CODE_SYSTEM_DEFAULTS_PATH
    effect: "Overrides the system defaults file path, which is the lowest settings-file layer above hardcoded defaults."
    effect_category: config_path_override
  - name: QWEN_CODE_TRUSTED_FOLDERS_PATH
    effect: "Overrides the trustedFolders.json path used when folder trust is enabled."
    effect_category: config_path_override
  - name: QWEN_CODE_SUPPRESS_YOLO_WARNING
    effect: "Suppresses the warning printed for headless YOLO runs without sandboxing; it does not change permissions."
    effect_category: none
  - name: QWEN_CODE_LEGACY_MCP_BLOCKING
    effect: "Restores older blocking MCP discovery behavior; this affects when tools become available, not approval decisions."
    effect_category: none
  - name: QWEN_CODE_FORCE_ENCRYPTED_FILE_STORAGE
    effect: "Forces encrypted/keychain-backed storage for MCP OAuth tokens where available instead of plaintext token files."
    effect_category: security_hardening
  - name: QWEN_TLS_INSECURE
    effect: "Disables TLS verification for API connections when truthy; this is a security-control escape hatch, not a tool permission."
    effect_category: security_hardening

config_files:
  - os: macos
    user: ".qwen/settings.json"
    repo: ".qwen/settings.json"
    notes: "User path is relative to the effective home or QWEN_HOME. System defaults: /Library/Application Support/QwenCode/system-defaults.json; system override: /Library/Application Support/QwenCode/settings.json. This machine has /Users/ken/.qwen/settings.json with auth/model/provider settings and no permissions block; the session HOME /Users/ken/.claudine has no .qwen/settings.json."
  - os: linux
    user: ".qwen/settings.json"
    repo: ".qwen/settings.json"
    notes: "User path is relative to the effective home or QWEN_HOME. System defaults: /etc/qwen-code/system-defaults.json; system override: /etc/qwen-code/settings.json."
  - os: windows
    user: ".qwen/settings.json"
    repo: ".qwen/settings.json"
    notes: "User path is relative to the Windows user home or QWEN_HOME. System defaults: C:\\ProgramData\\qwen-code\\system-defaults.json; system override: C:\\ProgramData\\qwen-code\\settings.json."

precedence:
  - source: "cli"
    scope: ["approval_mode", "rules", "tool_visibility", "mcp", "slash_commands", "extensions", "other"]
    merge_strategy: "none"
    notes: "CLI flags are session-scoped and generally override settings. --yolo and --approval-mode are mutually exclusive; use --approval-mode=yolo when the explicit mode flag is needed."
  - source: "env"
    scope: ["sandbox", "slash_commands", "config_loading", "mcp", "security_controls"]
    merge_strategy: "none"
    notes: "Environment variables override settings for their specific surfaces. QWEN_SANDBOX overrides both --sandbox and tools.sandbox, while sandbox image precedence is --sandbox-image, then QWEN_SANDBOX_IMAGE, then tools.sandboxImage."
  - source: "system_settings"
    scope: ["approval_mode", "rules", "mcp", "tool_visibility", "trust", "sandbox", "slash_commands"]
    merge_strategy: "deep"
    notes: "System settings override user and project settings. Admins can redirect the path with QWEN_CODE_SYSTEM_SETTINGS_PATH."
  - source: "project_config"
    scope: ["approval_mode", "rules", "mcp", "sandbox", "extensions", "hooks", "skills", "agents", "slash_commands"]
    merge_strategy: "deep"
    notes: "Project .qwen/settings.json overrides user settings when loaded. If folder trust is enabled and the folder is untrusted, project-local surfaces are ignored and privileged approval modes are forced down to default."
  - source: "user_config"
    scope: ["approval_mode", "rules", "mcp", "sandbox", "extensions", "hooks", "skills", "agents", "slash_commands"]
    merge_strategy: "deep"
    notes: "User settings apply globally and are overridden by project and system settings. Permission arrays merge by decision type rather than replacing wholesale."
  - source: "system_defaults"
    scope: ["approval_mode", "rules", "mcp", "sandbox", "tool_visibility"]
    merge_strategy: "deep"
    notes: "System defaults provide a base layer above hardcoded defaults and below user/project/system settings."
  - source: "rule_engine"
    scope: ["rules"]
    merge_strategy: "none"
    notes: "After sources are merged, runtime rule conflict priority is deny, then ask, then allow, then mode/default behavior."

default_posture: "With no relevant CLI flags, environment variables, settings files, or trust changes, Qwen Code starts in Ask Permissions mode (`tools.approvalMode: \"default\"`). Read-only and metadata tools run without confirmation, read-only shell commands are auto-allowed by shell analysis, and mutating shell/edit/network/MCP/subagent actions ask or follow their tool defaults."

cli_zero_permissions:
  supported: false
  invocation: "qwen --safe-mode --approval-mode plan --exclude-tools \"Bash,Shell,Edit,Write,NotebookEdit,WebFetch,Agent,Skill,Monitor,SaveMemory,ReadMcpResource,mcp__*\" --max-tool-calls 0 -p \"...\""
  mechanism: "Approximation only: safe mode disables config-sourced customizations, plan mode blocks non-info tools, exclude-tools denies named surfaces, and max-tool-calls 0 aborts on any tool call."
  limitations: "There is no CLI deny-all wildcard or empty tool allowlist that both starts from no permissions/no tools and then selectively adds tools back in the same run. --max-tool-calls 0 is a hard execution budget, not a mutable permission baseline; --core-tools cannot express an empty allowlist and is ignored in safe mode; --approval-mode plan still allows read/info tools."

agent_permissions:
  allowed: true
  fm_properties:
    - approvalMode
    - tools
    - disallowedTools

yolo:
  has_interactive_yolo: true
  has_non_interactive_yolo: true
  mechanism: "--yolo, --approval-mode yolo, /approval-mode yolo, Shift+Tab/Tab mode cycling, or tools.approvalMode: \"yolo\" in settings.json. Headless YOLO without sandbox prints a warning unless QWEN_CODE_SUPPRESS_YOLO_WARNING is set."

policy_engine:
  ergonomic: false
  provides_coverage: true
  gaps:
    - "Auto mode is classifier-driven and can fail closed or fall back to manual approval after consecutive blocks/unavailable results; this is not a deterministic static rule."
    - "Over-broad allow rules are temporarily stripped only while in Auto mode, without mutating settings.json."
    - "The rule grammar has Qwen-specific aliases, meta-categories, command splitting, command word-boundary semantics, path-prefix semantics, and shell virtual operations that require provider-specific matching."
    - "Tool visibility is separate from approval: coreTools registry filtering, whole-tool deny hiding, MCP include/exclude filters, extensions, and slash-command denylist."
    - "Folder trust and safe mode are source-loading gates, not simple rules."
    - "MCP policy includes server filters, per-server trust, resources, prompts as slash commands, OAuth token storage, and discovery timing."
    - "Sandbox mode has OS/container backends, network profiles, and image/proxy settings outside static permission rules."
    - "Subagent permission inheritance and parent-mode dominance, including yolo/auto-edit/plan override behavior, are runtime semantics."

permission_entities:
  - entity: tool
    native_names: ["permissions.allow", "permissions.ask", "permissions.deny", "--allowed-tools", "--exclude-tools", "tools.core", "tools.allowed", "tools.exclude"]
    notes: "Rules target built-in tool names, aliases, MCP tool names, and Tool(specifier) patterns. Legacy tools.allowed/exclude/core are deprecated and migrated or preserved for compatibility."
  - entity: tool_group
    native_names: ["Read", "Edit", "Bash"]
    notes: "Read covers read_file, grep_search, glob, and list_directory. Edit covers edit, write_file, and notebook_edit. Bash covers run_shell_command and monitor."
  - entity: command
    native_names: ["Bash(pattern)", "Shell(pattern)", "Monitor(pattern)"]
    notes: "Shell command patterns support * globs, word-boundary behavior when a space precedes *, prefix matching without *, compound command splitting, and virtual file/network operation extraction."
  - entity: path
    native_names: ["Read(path)", "ReadFile(path)", "Edit(path)", "Write(path)", "NotebookEdit(path)"]
    notes: "Path rules use // absolute, ~/ home-relative, / project-root-relative, ./ cwd-relative, and no-prefix cwd-relative patterns with picomatch/gitignore-style matching."
  - entity: workspace
    native_names: ["context.includeDirectories", "--include-directories", "--add-dir", "security.folderTrust.enabled"]
    notes: "Workspace and included directories affect read/write scope and context; folder trust can gate project-local settings and force approval mode down."
  - entity: mcp_server
    native_names: ["mcpServers", "mcp.allowed", "mcp.excluded", "--allowed-mcp-server-names", "--mcp-config"]
    notes: "Server names can be filtered by config or CLI. Project/workspace MCP servers can be gated; CLI/session-injected servers are top-tier and not gated."
  - entity: mcp_tool
    native_names: ["mcp__<server>", "mcp__<server>__<tool>", "includeTools", "excludeTools"]
    notes: "MCP tools use normal permission rules plus per-server include/exclude tool filters; excludeTools wins over includeTools."
  - entity: mcp_resource
    native_names: ["read_mcp_resource", "ReadMcpResource", "@server:uri"]
    notes: "MCP resources can be browsed and inserted into prompts; resource reads are disabled in untrusted folders."
  - entity: agent
    native_names: ["Agent", "agent", "Task"]
    notes: "The Agent tool spawns named subagents and fork subagents. Rules can target the tool or Agent(subagent_type)."
  - entity: subagent
    native_names: ["approvalMode", "tools", "disallowedTools", "--max-subagent-depth"]
    notes: "Subagent Markdown frontmatter controls model, approval mode, and tool allow/deny lists. Parent yolo/auto-edit/plan modes dominate narrower subagent settings."
  - entity: mode
    native_names: ["tools.approvalMode", "--approval-mode", "--yolo", "/approval-mode", "/plan"]
    notes: "Approval mode sets the baseline: plan, default, auto-edit, auto, or yolo."
  - entity: approval_category
    native_names: ["allow", "ask", "deny", "default"]
    notes: "Permission decisions are evaluated deny > ask > allow > default."
  - entity: sandbox
    native_names: ["tools.sandbox", "tools.sandboxImage", "--sandbox", "--sandbox-image", "QWEN_SANDBOX", "SEATBELT_PROFILE"]
    notes: "Sandboxing is a separate OS/container isolation layer from approval mode."
  - entity: hook
    native_names: ["hooks", ".qwen/hooks"]
    notes: "Hooks are disabled by safe mode and untrusted workspaces."
  - entity: extension
    native_names: ["extensions", "--extensions", "-e none"]
    notes: "Extensions can add tools, commands, skills, and subagents; safe mode disables them and CLI can select or disable them for a session."
  - entity: slash_command
    native_names: ["slashCommands.disabled", "--disabled-slash-commands", "QWEN_DISABLED_SLASH_COMMANDS"]
    notes: "Slash-command visibility is independent of tool permissions and is a union denylist across sources."

approval_modes:
  - name: plan
    effect: "Read-only planning/exploration. Blocks non-info tools except plan entry/exit and user-question tools."
    interactive: true
    non_interactive: true
    aliases: ["plan", "--approval-mode plan", "/approval-mode plan", "/plan"]
  - name: default
    effect: "Ask Permissions mode. Read-only operations and read-only shell commands can run; risky operations ask."
    interactive: true
    non_interactive: true
    aliases: ["default", "Ask Permissions", "--approval-mode default", "/approval-mode default"]
  - name: auto-edit
    effect: "Auto-approves edit/info confirmation types while shell commands and other risky tools still ask."
    interactive: true
    non_interactive: true
    aliases: ["auto-edit", "auto_edit", "autoedit", "--approval-mode auto-edit", "/approval-mode auto-edit"]
  - name: auto
    effect: "Classifier-driven approval. In-workspace edits and safe/read-only tools use fast paths; shell, network, out-of-workspace edits, MCP, and agent calls route through an LLM classifier unless hard rules decide first."
    interactive: true
    non_interactive: true
    aliases: ["auto", "--approval-mode auto", "/approval-mode auto", "tools.approvalMode: auto"]
  - name: yolo
    effect: "Auto-approves all tool calls except ask_user_question and hard denials/other guards. Does not automatically sandbox."
    interactive: true
    non_interactive: true
    aliases: ["yolo", "--yolo", "-y", "--approval-mode yolo", "/approval-mode yolo"]

rule_model:
  decisions: ["deny", "ask", "allow", "default"]
  syntax: "Rules are strings: ToolName or ToolName(specifier). Built-in aliases include Bash/Shell, Read/ReadFile, Edit/EditFile, Write/WriteFile, NotebookEdit, Grep, Glob, ListFiles, WebFetch, Agent/Task, Skill, ReadMcpResource, Lsp, Monitor, and MCP names like mcp__server__tool."
  precedence: "PermissionManager checks session deny, persistent deny, session ask, persistent ask, session allow, persistent allow, then default. For combined shell virtual operations, the most restrictive decision wins."
  merge_semantics: "Settings files are deep-merged by schema strategy. permissions.allow/ask/deny arrays merge across settings scopes; CLI allowed-tools appends session allow rules and exclude-tools appends session deny rules. Legacy tools.allowed/exclude are read for compatibility; tools.core is separate registry allowlist state."
  matcher_semantics: "Shell patterns use * globs with word-boundary behavior for patterns like `git *`, prefix matching when no * is present, and compound-command splitting. Path patterns use // absolute, ~/ home, / project-root, ./ cwd, and cwd-relative default semantics. WebFetch matches domains. Literal specifiers match agent, skill, and resource/server names. MCP tools are matched by their mcp__server__tool names."
  default_decision: "In default mode, each tool's default permission applies: safe/read-only tools are allowed, read-only shell commands are allowed by AST analysis, and risky shell/edit/network/MCP/agent operations ask. Plan blocks non-info tools; auto-edit auto-approves edit/info confirmations; auto uses classifier fast paths and fail-closed logic; yolo auto-approves unless a hard denial applies."

tool_visibility:
  supported: true
  mechanisms:
    - "--core-tools and legacy tools.core restrict registered core tools."
    - "A whole-tool deny rule without a specifier prevents a tool from being registered or visible."
    - "--exclude-tools blocks matching tools/rules for the session."
    - "mcp.allowed, mcp.excluded, --allowed-mcp-server-names, and per-server includeTools/excludeTools restrict MCP server/tool visibility."
    - "--extensions and `qwen -e none` control extension-provided surfaces."
    - "slashCommands.disabled, --disabled-slash-commands, and QWEN_DISABLED_SLASH_COMMANDS hide/refuse slash commands."
    - "Subagent tools/disallowedTools restrict a subagent's tool pool."
  notes: "Tool visibility and approval are distinct. An allow rule auto-approves matching calls; a registry allowlist or server/tool filter can prevent the model from seeing a tool at all."

sandbox:
  supported: true
  modes: ["permissive-open", "permissive-closed", "permissive-proxied", "restrictive-open", "restrictive-closed", "restrictive-proxied", "docker", "podman", "sandbox-exec"]
  backends: ["macOS Seatbelt sandbox-exec", "Docker container", "Podman container"]
  filesystem_control: "macOS default permissive-open restricts writes outside the project directory while allowing most other operations. Container sandboxing mounts the workspace and ~/.qwen so auth/settings persist. Include directories and custom sandbox profiles/images can widen access."
  network_control: "Seatbelt profiles distinguish open, closed, and proxied network behavior. Container network behavior depends on Docker/Podman flags and image setup. QWEN_SANDBOX_PROXY_COMMAND plus proxied profiles can implement allowlist-style egress."
  notes: "Sandboxing is opt-in and separate from approval mode. Docs state Qwen Code runs in a sandbox to reduce risk when tools execute shell commands or modify files, and that MCP servers/tools must be available inside the sandbox environment. If dependencies are missing, sandbox startup can fail with FatalSandboxError rather than silently becoming a permission rule."

trust_and_admin:
  folder_trust: "Folder trust is disabled by default and enabled with security.folderTrust.enabled. When enabled, trust choices are stored in trustedFolders.json; IDE trust signal has priority over the local trust file. Untrusted workspaces ignore project settings, project .env files, extension management, auto-acceptance, and automatic memory loading; current source also forces auto-edit/auto/yolo approval modes down to default while allowing default and plan."
  managed_policy: "System defaults and system settings files provide the admin-managed settings layers. System settings override user/project settings; admins can redirect these paths with QWEN_CODE_SYSTEM_DEFAULTS_PATH and QWEN_CODE_SYSTEM_SETTINGS_PATH."
  safe_mode: "--safe-mode or QWEN_CODE_SAFE_MODE=true disables customizations including context files, hooks, extensions, skills, MCP servers, custom subagents, settings-sourced permission rules and approval mode, memory features, and sandbox settings. Explicit CLI approval mode, yolo, allowed-tools, and exclude-tools continue to apply."
  notes: "The observed current user config at /Users/ken/.qwen/settings.json contains auth/model/provider settings only, not permissions. No trustedFolders.json exists in either /Users/ken/.qwen or the session's /Users/ken/.claudine/.qwen. The repository has .qwen/agents, commands, and skills but no repo .qwen/settings.json."

mcp_permissions:
  supported: true
  server_filters:
    - "mcp.allowed allowlists server names; glob * and ? are supported."
    - "mcp.excluded denies server names; excluded wins when a server is both allowed and excluded."
    - "--allowed-mcp-server-names is a session upper bound and causes mcp.allowed/excluded to be ignored."
    - "--mcp-config injects top-tier session MCP servers without mutating settings."
    - "Project/workspace MCP servers can be gated until approved; YOLO skips pending gating."
    - "Safe mode and bare mode disable settings-sourced MCP servers."
  tool_filters:
    - "mcpServers.<name>.includeTools allowlists tools from one server."
    - "mcpServers.<name>.excludeTools denies tools from one server and wins over includeTools."
    - "permissions.allow/ask/deny can target mcp__server and mcp__server__tool names."
    - "Auto mode usually blocks MCP tools unless explicitly allowed or the tool implements safe classifier projection."
  trust_model: "Per-server trust: true bypasses all tool-call confirmations for that server. Folder trust gates project-local MCP resource reads and project/workspace MCP configuration when enabled. MCP OAuth tokens default to ~/.qwen/mcp-oauth-tokens.json mode 0600 unless encrypted storage is forced."
  notes: "MCP exposes tools, prompts as slash commands, and resources. Resources are inserted with @server:uri and are disabled in untrusted folders. Current docs say sandboxed sessions require MCP server commands to be available inside the sandbox environment."

headless_behavior: "Headless mode uses -p/--prompt or piped stdin. Interactive permission prompts cannot be shown in ordinary text/json headless output; use --approval-mode plan/yolo/auto, --allowed-tools, --exclude-tools, stream-json/SDK permission channels, or run budgets. Source shows teammate approval that needs a prompt fails in non-stream-json modes and instructs callers to use --yolo or stream-json."

approval_persistence: "CLI flags and session rules do not persist. Interactive confirmation outcomes can persist allow rules to project or user settings via ProceedAlwaysProject/ProceedAlwaysUser, and the in-memory PermissionManager is updated immediately. Persisted rules use permissions.allow entries scoped to the generated rule pattern."

protected_paths:
  - ".qwen/settings*.json"
  - ".qwen/rules/"
  - ".qwen/commands/"
  - ".qwen/agents/"
  - ".qwen/skills/"
  - ".qwen/hooks/"
  - ".mcp.json"
  - "QWEN.md"
  - "AGENTS.md"
  - "QWEN.local.md"
  - "configured context filenames"
  - ".git/"
  - ".husky/"
  - "package.json"
  - ".npmrc"
  - "Makefile"
  - ".github/workflows/"
  - "symlinks targeting protected paths"

security_posture: "Qwen Code combines client-side static permission rules, advisory/interactive approval UX, classifier-based Auto mode, optional OS/container sandboxing, and managed settings/trust gates. Only the sandbox layer is an OS/container enforcement boundary; permission rules, tool visibility, safe mode, and managed settings are client-side controls."

changes:
  - "Refreshed against upstream Qwen Code 0.19.6 docs/source and compared with the locally installed 0.15.6 CLI."
  - "Updated --approval-mode to include current CLI value auto; prior research said auto was not a CLI value."
  - "Restored --safe-mode as a current documented and implemented CLI flag, and described its interaction with explicit CLI permission flags."
  - "Captured that folder trust is disabled by default and, when enabled, untrusted workspaces force privileged modes down to default."
  - "Updated legacy tools.core/tools.allowed/tools.exclude status: they are deprecated/migrated compatibility surfaces, while permissions.allow/ask/deny are the preferred grammar."
  - "Added max-tool-calls as the only CLI-only hard no-tool-execution budget, while keeping cli_zero_permissions unsupported because it cannot selectively add permissions back."
  - "Updated MCP coverage for resources, prompts-as-slash-commands, per-server include/exclude, OAuth token storage, progressive discovery, and sandbox availability requirements."
  - "Updated sandbox details for QWEN_SANDBOX precedence, sandbox image precedence, Seatbelt profiles, proxying, and the fact that YOLO does not imply sandboxing."
  - "Recorded local config inspection: /Users/ken/.qwen/settings.json exists with no permissions block; no settings.json exists in the session QWEN home and no repo .qwen/settings.json exists."

requires_claudine_update: true
reason: "Current Qwen Code permission metadata differs from the prior catalog in ways Claudine should model: --approval-mode auto is a CLI value, --safe-mode is a live security-control switch, folder trust is opt-in but constrains privileged modes when enabled, legacy tool keys are deprecated/migrated, max-tool-calls can act as a hard no-tool-execution budget, and MCP resources/prompts/trust/OAuth storage plus sandbox precedence need provider metadata coverage."
---

# Qwen CLI Permissions and Security Controls

## Introduction to Qwen CLI Permissions

Qwen Code permissions are defined by five cooperating layers:

- **Approval mode**: `plan`, `default`, `auto-edit`, `auto`, or `yolo`.
- **Permission rules**: `permissions.allow`, `permissions.ask`, and `permissions.deny`.
- **Tool visibility controls**: core-tool allowlists, deny rules without specifiers, MCP server/tool filters, extension selection, and disabled slash commands.
- **Trust and safe-mode gates**: folder trust and safe mode decide whether project/user customizations are loaded.
- **Sandboxing**: optional OS/container isolation separate from approval decisions.

The main configuration file grammar is JSON in `settings.json`. User-scope settings live at `~/.qwen/settings.json` or under `QWEN_HOME`; project settings live at `.qwen/settings.json`; system defaults and system settings provide admin layers. Current Qwen Code also supports `.mcp.json` and session-injected MCP config, but permission rules themselves are in `settings.json`.

Current preferred permission rules are:

```json
{
  "tools": {
    "approvalMode": "default"
  },
  "permissions": {
    "allow": ["Read", "Bash(npm test)", "WebFetch(docs.rs)"],
    "ask": ["Bash(git push *)", "Edit"],
    "deny": ["Bash(rm -rf *)", "Read(.env)", "mcp__untrusted"]
  }
}
```

Legacy `tools.allowed`, `tools.exclude`, and `tools.core` still exist for compatibility, but the docs mark them deprecated. `tools.allowed` maps to `permissions.allow`, `tools.exclude` maps to `permissions.deny`, and `tools.core` retains registry allowlist semantics for core tools.

Environment variables influence adjacent security controls: `QWEN_SANDBOX`, `QWEN_SANDBOX_IMAGE`, `SEATBELT_PROFILE`, `QWEN_CODE_SAFE_MODE`, `QWEN_DISABLED_SLASH_COMMANDS`, `QWEN_HOME`, and system settings path overrides are the important ones. `QWEN_CODE_SUPPRESS_YOLO_WARNING` only suppresses a warning; it does not grant or remove permissions.

The important CLI switches are in the frontmatter. Precedence is mostly CLI above environment above settings, but sandboxing has documented exceptions: `QWEN_SANDBOX` overrides `--sandbox` and `tools.sandbox`, while sandbox image selection is `--sandbox-image` over `QWEN_SANDBOX_IMAGE` over `tools.sandboxImage`.

Approval policy is not the same as tool visibility. `permissions.allow` pre-approves a visible tool call. A bare `permissions.deny` rule, `--core-tools`, `mcp.allowed`, `mcp.excluded`, per-server `includeTools`/`excludeTools`, extension selection, and slash-command disabling can remove or hide capabilities before the model can use them.

## Permissions Use Cases

### Default

With no permission-relevant CLI flags, environment variables, config files, or trust state, Qwen Code uses `tools.approvalMode: "default"`, now presented as **Ask Permissions** mode. Read-only built-ins and read-only shell commands run without confirmation. Mutating edits, non-read-only shell commands, web fetches, MCP calls, subagent spawns, skills, memory writes, and similar actions ask according to their tool defaults.

PolicyEngine can describe the coarse default as allow read/info, ask write/execute/network/MCP/agent, and deny nothing. That is usable but not ergonomic for exact Qwen behavior because Qwen's shell read-only AST check, command splitting, virtual file/network operations, and tool aliases need a provider-specific matcher.

If no PolicyEngine changes were made, Claudine could define this use case at a conservative level. It would lose precision around read-only shell commands, meta-categories, and Qwen's generated "always allow" rule scopes.

### Whitelisting

Qwen Code has no single CLI or config wildcard that means "deny every possible tool, then add back a few." The practical approaches are:

- **Interactive whitelist style**: run in `default` mode, deny high-risk categories, and add `allow` rules only for known-safe operations.
- **Read-only planning**: run with `--approval-mode plan`, knowing read/info tools still work.
- **Hard no-tool execution**: run with `--max-tool-calls 0`, knowing the first tool call aborts and no permissions can be added back in that run.
- **Visibility reduction**: use `--safe-mode`, `--extensions none`, `--allowed-mcp-server-names`, `--exclude-tools`, and optionally `--core-tools` outside safe mode.

Best CLI-only, session-scoped locked-down invocation for a future Claudine wrapper is an approximation, not a true add-back baseline:

```bash
qwen --safe-mode \
  --approval-mode plan \
  --exclude-tools "Bash,Shell,Edit,Write,NotebookEdit,WebFetch,Agent,Skill,Monitor,SaveMemory,ReadMcpResource,mcp__*" \
  --max-tool-calls 0 \
  -p "..."
```

This is useful for "answer without tools" enforcement, but it is not a good foundation for "start with no tools and add exactly these tools back" because `--max-tool-calls 0` aborts all tools and `--core-tools` is ignored in safe mode.

Examples for adding limited permissions:

```bash
qwen -p "summarize the repo" \
  --approval-mode plan \
  --allowed-tools "Read,Grep,Glob,ListFiles"
```

```bash
qwen -p "run tests only" \
  --approval-mode default \
  --allowed-tools "Bash(npm test),Read" \
  --exclude-tools "Edit,Write,NotebookEdit,WebFetch,Agent,Skill,mcp__*"
```

```bash
qwen -p "use only the github MCP server" \
  --allowed-mcp-server-names github \
  --allowed-tools "mcp__github__get_issue,Read" \
  --exclude-tools "Bash,Edit,Write"
```

PolicyEngine can express the intended whitelist with explicit allow/ask/deny rules, but it cannot produce a native Qwen "deny everything then add back" CLI invocation because Qwen does not expose that primitive. A provider backend would need to enumerate Qwen categories, understand MCP wildcards, and warn that full zero-permissions is unsupported except by the blunt tool-call budget.

### YOLO

YOLO can be enabled with `--yolo`, `-y`, `--approval-mode yolo`, `/approval-mode yolo`, keyboard mode cycling, or `tools.approvalMode: "yolo"` in settings. It is available in interactive and non-interactive runs.

In YOLO mode, tool calls are auto-approved, including file edits, shell commands, web fetches, MCP tools, skills, and subagents. It does not enable sandboxing. Headless YOLO without sandbox prints a warning unless `QWEN_CODE_SUPPRESS_YOLO_WARNING` is set.

YOLO does not bypass all controls. Explicit deny rules, tool visibility filters, safe mode source gates, untrusted-folder mode downgrades, run budgets, and OS/container sandbox restrictions can still matter. Source also keeps `ask_user_question` outside the blanket YOLO auto-approve path.

### Root User

I found no current Qwen documentation or obvious source check that treats root/sudo sessions specially for approval mode. That means there is no documented root-specific YOLO prohibition comparable to some other providers. The safe statement for Claudine metadata is: Qwen Code has no documented root permission mode change; root sessions run with the process privileges they were launched with, so YOLO as root is especially dangerous unless sandboxed or blocked by admin policy.

### Configuring the Default

Default permissions are configured in:

- **User scope**: `~/.qwen/settings.json` or `$QWEN_HOME/settings.json`.
- **Repo/project scope**: `.qwen/settings.json`.
- **System defaults**: `/etc/qwen-code/system-defaults.json`, `C:\ProgramData\qwen-code\system-defaults.json`, or `/Library/Application Support/QwenCode/system-defaults.json`.
- **System override**: `/etc/qwen-code/settings.json`, `C:\ProgramData\qwen-code\settings.json`, or `/Library/Application Support/QwenCode/settings.json`.

Example user default:

```json
{
  "tools": {
    "approvalMode": "auto-edit"
  },
  "permissions": {
    "allow": ["Read", "Bash(git status)", "Bash(npm test)"],
    "ask": ["Bash(git push *)"],
    "deny": ["Read(./.env)", "Read(~/\\.ssh/**)", "WebFetch(malicious.com)"]
  }
}
```

Example project default:

```json
{
  "tools": {
    "approvalMode": "default",
    "sandbox": true
  },
  "permissions": {
    "allow": ["Bash(npm run lint)", "Bash(npm test)"],
    "deny": ["Bash(curl * | sh)", "Edit(.github/workflows/**)"]
  },
  "mcp": {
    "allowed": ["github", "docs-*"],
    "excluded": ["experimental-*"]
  }
}
```

Example Auto mode hints:

```json
{
  "tools": {
    "approvalMode": "auto"
  },
  "permissions": {
    "autoMode": {
      "hints": {
        "allow": ["Running pytest and ruff in this Python project"],
        "softDeny": ["Editing Qwen Code settings unless explicitly requested"],
        "hardDeny": ["Sending secrets or .env contents to any network endpoint"]
      },
      "environment": ["Private monorepo; production credentials are not in files"],
      "classifyAllShell": true
    }
  }
}
```

### Extending the Base

User settings can define a broad base and project settings can narrow it:

```json
{
  "permissions": {
    "allow": ["Bash(curl *)"]
  }
}
```

```json
{
  "permissions": {
    "deny": ["Bash(curl *)"]
  }
}
```

The repo deny wins because deny is evaluated before allow after merge.

A CLI mode can override config for one run:

```bash
qwen -p "plan a migration" --approval-mode plan
```

This starts in plan mode even if user settings default to `auto-edit` or `auto`.

Sandbox has a special precedence caveat:

```bash
QWEN_SANDBOX=false qwen -s -p "run tests"
```

Docs state `QWEN_SANDBOX` overrides the CLI sandbox flag and settings, so Claudine should not assume `-s` always wins when the environment is set.

## Tools and Permissions

Current Qwen Code built-in tools include:

| Tool or family | Default posture | Notes |
| --- | --- | --- |
| `read_file` / `ReadFile` | Allow | Read rules can cover this directly or through `Read`. |
| `grep_search` / `Grep` | Allow | Covered by `Read`. |
| `glob` / `Glob` | Allow | Covered by `Read`. |
| `list_directory` / `ListFiles` | Allow | Covered by `Read`. |
| `edit` / `Edit` | Ask or mode-dependent | Covered by `Edit`. |
| `write_file` / `WriteFile` | Ask or mode-dependent | Covered by `Edit` or `Write`. |
| `notebook_edit` / `NotebookEdit` | Ask or mode-dependent | Covered by `Edit`. |
| `run_shell_command` / `Bash` / `Shell` | Read-only allow, otherwise ask | Command AST and permission rules decide. |
| `monitor` | Read-only/command-dependent | Long-lived shell commands; `Bash` rules also cover monitor. |
| `web_fetch` / `WebFetch` | Ask or classifier-dependent | Domain rules can allow/ask/deny. |
| `agent` / `Agent` | Ask or mode-dependent | Spawns subagents; can target `Agent(type)`. |
| `skill` / `Skill` | Ask or mode-dependent | Skills can be disabled by safe mode/trust/visibility. |
| `todo_write` / `TodoList` | Usually allow | Metadata/session checklist. |
| `save_memory` / `SaveMemory` | Ask or setting-dependent | Persists memory. |
| `exit_plan_mode` / `enter_plan_mode` | Mode control | Plan-mode workflow tools. |
| `ask_user_question` | Special | Not blanket-auto-approved by YOLO code path. |
| `lsp` / `Lsp` | Trust-dependent | LSP servers are trust-gated by default. |
| `cron_*`, `loop_wakeup`, `workflow`, `artifact`, `computer_use__*` | Feature/mode-dependent | Scheduled, workflow, artifact, and desktop automation surfaces. |
| `read_mcp_resource` / `ReadMcpResource` | Trust/resource-dependent | Reads MCP resources; disabled in untrusted folders. |
| `mcp__<server>__<tool>` | Ask unless allowed/trusted/mode-dependent | External tools from MCP servers. |
| `tool_search` | Visibility/search tool | Can defer/hide tool discovery and is denied for some model heuristics. |

Permissions map to tool calls through `PermissionManager`. Each tool produces a default decision, then rules refine it. The rule engine checks deny, ask, allow, then falls back to mode/tool default. Shell commands receive extra analysis: Qwen splits compound commands, detects read-only shell commands, and extracts virtual read/write/network operations so path and WebFetch rules cannot be bypassed with simple shell equivalents such as `cat .env` or `curl`.

Rule grammar:

- Decision values: `allow`, `ask`, `deny`, plus internal/default fallback.
- Rule forms: `ToolName` or `ToolName(specifier)`.
- Command examples: `Bash(git *)`, `Bash(npm test)`, `Monitor(pnpm dev)`.
- Path examples: `Read(./secrets/**)`, `ReadFile(./.env)`, `Edit(/src/**/*.ts)`, `Read(//etc/passwd)`.
- Domain examples: `WebFetch(api.example.com)`.
- Literal examples: `Agent(cautious-reviewer)`, `Skill(pdf)`, `ReadMcpResource(server)`.
- MCP examples: `mcp__github`, `mcp__github__get_issue`, `mcp__*`.

Conflict precedence is deny before ask before allow. The first matching rule in the current decision class wins, with session rules checked before persistent rules inside each class.

Approvals can persist when a user selects project/user "always allow" outcomes. Qwen writes generated `permissions.allow` rules to project or user `settings.json` and updates the active PermissionManager immediately. Session CLI flags do not persist.

## Sandboxing, Trust, and Administrative Controls

Sandboxing is separate from approval mode. `--yolo` does not imply `--sandbox`.

Backends:

- **macOS**: Seatbelt via `sandbox-exec`; default profile is `permissive-open`.
- **Linux/Windows**: Docker or Podman containers.
- **Container image**: built-in package image unless `--sandbox-image`, `QWEN_SANDBOX_IMAGE`, or `tools.sandboxImage` selects another.

Filesystem and network controls depend on backend/profile. Seatbelt profiles distinguish write restrictions and network-open/closed/proxied behavior. Container sandboxing mounts the workspace and `~/.qwen`; custom Dockerfiles, `.qwen/sandbox.bashrc`, `SANDBOX_FLAGS`, and proxy commands can alter behavior.

Folder trust is disabled by default. When enabled, trust decisions are stored in `trustedFolders.json`; an IDE trust signal takes priority if present. Untrusted workspaces ignore project settings and `.env` files, disable extension management, disable auto-acceptance, and disable automatic memory loading. Current source also forces non-default/non-plan approval modes down to `default` when the folder is not trusted.

Managed/admin policy uses system defaults and system settings. System settings are the strongest settings-file layer. They merge/override through the settings loader but remain client-side controls; they are not an OS sandbox.

Protected paths are especially important in Auto mode. Qwen routes writes to Qwen self-modification surfaces and persistence surfaces through the classifier even when they are inside the workspace, and symlinks targeting protected paths are rejected. The frontmatter `protected_paths` list captures the documented examples.

Security posture: default Qwen permissions are advisory/client-side policy plus interactive approval. Auto mode adds an LLM classifier. System settings and safe mode constrain client behavior. The sandbox is the OS/container enforcement boundary.

## MCP and Permissions

Qwen MCP support covers tools, prompts, and resources. Tools appear as `mcp__<server>__<tool>` and participate in permission rules. Prompts become slash commands labeled by server. Resources can be browsed and referenced in prompts with `@server:uri`; resource reads are disabled in untrusted folders.

Safer MCP configuration layers:

- Use `mcp.allowed` and `mcp.excluded` to restrict server names.
- Use `--allowed-mcp-server-names` for a session-scoped upper bound.
- Use per-server `includeTools` and `excludeTools`; exclude wins over include.
- Avoid `trust: true` unless the server is fully trusted because it bypasses all tool confirmations for that server.
- Use `permissions.deny` for high-risk MCP tool names such as `mcp__filesystem__write_file`.
- Use `--mcp-config` to inject session-only servers without mutating user/project config.
- Use safe mode or bare mode to disable settings-sourced MCP servers.

MCP OAuth tokens are stored in `~/.qwen/mcp-oauth-tokens.json` by default with mode 0600. `QWEN_CODE_FORCE_ENCRYPTED_FILE_STORAGE=true` asks Qwen to use keychain-backed or AES-GCM encrypted storage where available.

Current docs state that in sandbox mode, tools including MCP servers must be available inside the sandbox environment. That is a change from the older assumption that MCP tools always run outside the sandbox. Claudine should model this as provider-version-sensitive: Qwen's sandbox wraps the execution environment, but remote MCP services and OAuth/token behavior still need separate trust modeling.

## Non-Interactive Behavior

Headless mode is entered with `--prompt`/`-p`, positional prompt in non-TTY, or piped stdin. Plain headless output cannot show ordinary interactive prompts. Safe choices are `--approval-mode plan`, explicit `--allowed-tools`, `--approval-mode auto`, or `--approval-mode yolo` with sandboxing and run budgets.

Stream-json and SDK/daemon paths expose programmatic permission channels. Developer docs describe permission request/resolution events and a `canUseTool` callback with timeout/auto-deny behavior for SDKs. Source comments show non-stream-json teammate approvals cannot prompt and fail with guidance to use `--yolo` or stream-json.

## Sources

- [Qwen Code user overview](https://qwenlm.github.io/qwen-code-docs/en/users/overview)
- [Qwen Code settings docs](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/settings)
- [Qwen Code approval mode docs](https://qwenlm.github.io/qwen-code-docs/en/users/features/approval-mode)
- [Qwen Code auto mode docs](https://qwenlm.github.io/qwen-code-docs/en/users/features/auto-mode)
- [Qwen Code sandbox docs](https://qwenlm.github.io/qwen-code-docs/en/users/features/sandbox)
- [Qwen Code MCP docs](https://qwenlm.github.io/qwen-code-docs/en/users/features/mcp)
- [Qwen Code headless docs](https://qwenlm.github.io/qwen-code-docs/en/users/features/headless)
- [Qwen Code trusted folders docs](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/trusted-folders)
- [Qwen Code subagents docs](https://qwenlm.github.io/qwen-code-docs/en/users/features/sub-agents)
- [QwenLM/qwen-code GitHub repository](https://github.com/QwenLM/qwen-code)
- [Qwen Code SDK TypeScript docs](https://github.com/QwenLM/qwen-code/blob/main/docs/developers/sdk-typescript.md)
- [Qwen Code serve protocol docs](https://github.com/QwenLM/qwen-code/blob/main/docs/developers/qwen-serve-protocol.md)
- Local clone of `QwenLM/qwen-code` at commit fetched 2026-07-03, package version 0.19.6.
- Local installed Qwen CLI at `/opt/homebrew/bin/qwen`, version 0.15.6, used for comparison.
- Local observed config: `/Users/ken/.qwen/settings.json`, `/Users/ken/.claudine/.qwen`, and repository `.qwen/`.

## Changelog

- 2026-07-03: Refreshed against upstream Qwen Code 0.19.6 docs/source and local 0.15.6 install. Updated `--approval-mode` to include `auto`, restored `--safe-mode`, documented opt-in folder trust, updated legacy permission key status, added `max-tool-calls` no-tool budget caveat, expanded MCP resources/prompts/OAuth/sandbox coverage, and recorded local config inspection.
- 2026-07-02: Refreshed research against Qwen Code 0.15.6 and current documentation. Corrected CLI flags (removed `--include-tools` and `--safe-mode`; added `--core-tools`, `--allowed-mcp-server-names`, and `--bare`). Limited `--approval-mode` CLI choices to `plan`/`default`/`auto-edit`/`yolo` and noted `auto` is available via settings.json and `/approval-mode`. Added schema-required frontmatter fields and updated the PolicyEngine assessment. Flagged Claudine backend/mutation updates as required.
