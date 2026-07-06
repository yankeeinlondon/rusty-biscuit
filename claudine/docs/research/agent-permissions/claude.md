---
$schema: ./_schema.yaml
created: 2026-07-01
last_updated: 2026-07-03
agent: codex
model: default

cli_params:
  - param: permission-mode
    style: switch
    description: "Begin the session in the specified permission mode. Values are default, acceptEdits, plan, auto, dontAsk, and bypassPermissions. Overrides the defaultMode setting for this session."
    example: "claude --permission-mode plan"
    example_description: "Starts an interactive planning session where file edits are never auto-approved."
  - param: allowedTools
    style: switch
    description: "Add allow rules for the session. Matching tool calls execute without prompting. Accepts the same Tool(specifier) syntax used in settings.json."
    example: 'claude --allowedTools "Bash(npm run *),Read,Edit"'
    example_description: "Auto-approves npm run commands and all Read/Edit calls for the session."
  - param: disallowedTools
    style: switch
    description: "Add deny rules for the session. A bare tool name removes the tool from the model's context; a scoped rule blocks matching calls. Accepts Tool(specifier) syntax."
    example: 'claude --disallowedTools "Agent(Explore),Bash(rm *)"'
    example_description: "Disables the Explore subagent and blocks rm commands while leaving other Bash calls available."
  - param: dangerously-skip-permissions
    style: switch
    description: "Equivalent to --permission-mode bypassPermissions. Skips permission prompts for the session. Refused when running as root on macOS/Linux outside a recognized sandbox."
    example: 'claude -p --dangerously-skip-permissions "deploy to staging"'
    example_description: "Runs a non-interactive deployment prompt with all permission prompts auto-approved."
  - param: allow-dangerously-skip-permissions
    style: switch
    description: "Adds bypassPermissions to the interactive Shift+Tab mode cycle without starting in it. Useful when you want to begin in another mode and switch later."
    example: "claude --permission-mode plan --allow-dangerously-skip-permissions"
    example_description: "Starts in plan mode but lets you cycle into bypassPermissions later via Shift+Tab."
  - param: agent
    style: switch
    description: "Select the agent/subagent definition for the current session, overriding the agent setting."
    example: "claude --agent code-reviewer"
    example_description: "Starts a session using the named agent definition and its tool restrictions."
  - param: agents
    style: switch
    description: "Define custom subagents dynamically as JSON for the current session. Accepts the same field names as subagent frontmatter plus prompt."
    example: 'claude --agents ''{"reviewer":{"description":"Reviews code","tools":"Read,Grep","prompt":"Review this change."}}'''
    example_description: "Adds a session-scoped reviewer agent with a restricted tool list."
  - param: add-dir
    style: switch
    description: "Add additional working directories that Claude may read and edit. Files in these directories follow the same permission rules as the launch directory."
    example: "claude --add-dir ../shared ../docs"
    example_description: "Grants access to sibling directories for this session only."
  - param: tools
    style: switch
    description: "Restrict which built-in tools Claude can use. Pass a comma-separated list, default for the full set, or an empty string to disable all built-in tools. MCP tools are not constrained by this flag."
    example: 'claude --tools "Bash,Read,Edit"'
    example_description: "Limits the session to Bash, Read, and Edit tools."
  - param: mcp-config
    style: switch
    description: "Load MCP servers from a JSON file or inline JSON string. Servers added this way are available for the session."
    example: "claude --mcp-config ./mcp.json"
    example_description: "Loads MCP servers defined in a project-local configuration file."
  - param: strict-mcp-config
    style: switch
    description: "Only use MCP servers provided via --mcp-config, ignoring user, project, plugin, and claude.ai connector servers."
    example: "claude --strict-mcp-config --mcp-config ./ci-mcp.json"
    example_description: "Runs a locked-down session where only the explicitly supplied MCP servers load."
  - param: channels
    style: switch
    description: "Enable MCP channel notification sources for the session. Takes space-separated plugin channel identifiers and requires claude.ai authentication."
    example: "claude --channels plugin:alerts@team-marketplace"
    example_description: "Lets the named channel plugin push events into this session."
  - param: dangerously-load-development-channels
    style: switch
    description: "Enable non-allowlisted development channels for local testing. Accepts plugin:<name>@<marketplace> and server:<name> entries and prompts for confirmation."
    example: "claude --dangerously-load-development-channels server:webhook"
    example_description: "Temporarily loads a local development channel source."
  - param: permission-prompt-tool
    style: switch
    description: "In non-interactive mode, route permission prompts to the named MCP tool for programmatic approval."
    example: 'claude -p --permission-prompt-tool mcp_auth_tool "query"'
    example_description: "Delegates permission decisions to an MCP tool during a headless run."
  - param: safe-mode
    style: switch
    description: "Disables customizations such as CLAUDE.md, skills, plugins, hooks, MCP servers, custom commands/agents, output styles, workflows, themes, keybindings, status line, file-suggestion commands, LSP, and auto-memory. Built-in tools and permissions continue to work normally."
    example: "claude --safe-mode"
    example_description: "Starts a session free of project customizations while keeping the permission system intact."
  - param: bare
    style: switch
    description: "Minimal mode that skips auto-discovery of hooks, skills, plugins, MCP servers, auto memory, and CLAUDE.md. Keeps Bash, Read, and Edit tools. Useful for CI and scripts."
    example: 'claude --bare -p "Summarize this file" --allowedTools "Read"'
    example_description: "Runs a headless summary task with no project configuration loaded."
  - param: setting-sources
    style: switch
    description: "Comma-separated list of setting sources to load: user, project, local. Useful for locking a session to a specific config tier."
    example: "claude --setting-sources user,project"
    example_description: "Prevents local project overrides from loading for this session."
  - param: settings
    style: switch
    description: "Path to a settings JSON file or an inline JSON string. Values override the same keys in settings.json for this session; omitted keys keep their file-based values."
    example: "claude --settings ./ci-settings.json"
    example_description: "Applies a session-specific settings overlay without modifying persisted config files."
  - param: worktree
    style: switch
    description: "Start Claude in an isolated git worktree under <repo>/.claude/worktrees/<name>. This isolates repository edits but is not a general OS sandbox."
    example: "claude --worktree feature-auth"
    example_description: "Runs the session in a dedicated git worktree for the current repository."

env_vars:
  - name: CLAUDE_CODE_ENABLE_AUTO_MODE
    effect: "Set to 1 to make auto mode available on Amazon Bedrock, Google Cloud Vertex AI, Microsoft Foundry, and signed-in Claude apps gateway sessions. Auto mode is available by default on the Anthropic API. Requires v2.1.158+."
    effect_category: approval_mode
  - name: CLAUDE_CODE_MCP_ALLOWLIST_ENV
    effect: "Set to 1 to spawn stdio MCP servers with only a safe baseline environment plus the server's configured env, rather than inheriting the user's full shell environment."
    effect_category: security_hardening
  - name: CLAUDE_CODE_SUBPROCESS_ENV_SCRUB
    effect: "Set to 1 to strip Anthropic and cloud-provider credentials from subprocess environments (Bash tool, hooks, MCP stdio servers). On Linux, also runs Bash subprocesses in an isolated PID namespace."
    effect_category: security_hardening
  - name: CLAUDE_CODE_SIMPLE
    effect: "Set to 1 to run with a minimal system prompt and only Bash, file read, and file edit tools. Disables auto-discovery of hooks, skills, plugins, MCP servers, auto memory, and CLAUDE.md. Equivalent to --bare."
    effect_category: customization_lockdown
  - name: CLAUDE_CODE_SAFE_MODE
    effect: "Set to 1 to start in safe mode. Disables CLAUDE.md, skills, plugins, hooks, MCP servers, custom commands/agents, output styles, workflows, themes, keybindings, status line, file-suggestion commands, LSP, and auto-memory. Permissions work normally."
    effect_category: customization_lockdown
  - name: ENABLE_CLAUDEAI_MCP_SERVERS
    effect: "Set to false to disable claude.ai MCP connectors for the session. Same effect as the disableClaudeAiConnectors setting, but does not affect servers passed via --mcp-config."
    effect_category: tool_surface
  - name: CLAUDE_CODE_PROVIDER_MANAGED_BY_HOST
    effect: "Set by host platforms that embed Claude Code and manage model provider routing. When set, provider-selection, endpoint, and auth variables in settings files are ignored."
    effect_category: other
  - name: CLAUDE_CODE_DISABLE_POLICY_SKILLS
    effect: "Set to 1 to skip loading skills from the system-wide managed skills directory, useful for container/CI sessions."
    effect_category: config_source_toggle
  - name: CLAUDE_CODE_PERFORCE_MODE
    effect: "Set to 1 to enable Perforce-aware write protection, preventing edits on files lacking the owner-write bit."
    effect_category: security_hardening
  - name: CLAUDE_CODE_POWERSHELL_RESPECT_EXECUTION_POLICY
    effect: "Set to 1 to stop bypassing PowerShell execution policy for tool calls, hooks, and status line commands."
    effect_category: security_hardening
  - name: CLAUDE_AGENT_SDK_DISABLE_BUILTIN_AGENTS
    effect: "Set to 1 in non-interactive mode (-p) to disable all built-in subagent types such as Explore and Plan."
    effect_category: tool_surface
  - name: CLAUDE_CODE_MCP_TOOL_IDLE_TIMEOUT
    effect: "Idle timeout in milliseconds for remote MCP tool calls before the call aborts (default 300000). Affects MCP execution, not permission policy."
    effect_category: none

config_files:
  - os: macos
    user: .claude/settings.json
    repo: .claude/settings.json
    notes: "User path is relative to $HOME. Local project overrides live in .claude/settings.local.json. User/local MCP state is stored in ~/.claude.json; project MCP servers are stored in .mcp.json. Endpoint-managed settings can also be delivered by macOS MDM profile or managed-settings.json."
  - os: linux
    user: .claude/settings.json
    repo: .claude/settings.json
    notes: "User path is relative to $HOME. Local project overrides live in .claude/settings.local.json. User/local MCP state is stored in ~/.claude.json; project MCP servers are stored in .mcp.json. Endpoint-managed settings can also be delivered by managed-settings.json."
  - os: windows
    user: .claude/settings.json
    repo: .claude/settings.json
    notes: "User path is relative to the Windows user profile directory. Local project overrides live in .claude/settings.local.json. User/local MCP state is stored in the profile's .claude.json; project MCP servers are stored in .mcp.json. Endpoint-managed settings can also be delivered by Windows registry policy or managed-settings.json."

precedence:
  - source: managed settings
    scope: [rules, sandbox, mcp, hooks, approval_mode, tool_visibility, agents]
    merge_strategy: none
    notes: "Server-managed (claude.ai admin console) and endpoint-managed (MDM plist, Windows registry, system managed-settings.json) sources cannot be overridden by CLI, env, or user/project/local settings. Within managed sources, a policyHelper preempts all other managed sources; otherwise server-managed wins over endpoint-managed. Permission deny rules still merge across all scopes and always beat allow rules."
  - source: cli
    scope: [approval_mode, rules, tool_visibility, workspace, config_loading, mcp, general_config]
    merge_strategy: none
    notes: "CLI flags are temporary session overrides. They cannot override managed settings. For permissions, --allowedTools and --disallowedTools add rules that are evaluated alongside file-based rules."
  - source: environment variables
    scope: [approval_mode, config_loading, security_controls, mcp]
    merge_strategy: none
    notes: "Where an env var and a settings field address the same behavior, the env var takes precedence over the settings field. CLI flags may still override the env var for the same behavior depending on the feature."
  - source: local project settings
    scope: [rules]
    merge_strategy: shallow
    notes: ".claude/settings.local.json. Permission allow/ask/deny arrays merge across local/project/user scopes; a deny rule from any scope overrides an allow rule from any scope."
  - source: local project settings
    scope: [general_config]
    merge_strategy: none
    notes: "Scalars and most objects replace narrower scopes; sandbox arrays merge across scopes unless managed-only locks apply."
  - source: shared project settings
    scope: [rules]
    merge_strategy: shallow
    notes: ".claude/settings.json. Permission arrays merge with user/local rules."
  - source: shared project settings
    scope: [general_config]
    merge_strategy: none
    notes: "Overrides user settings for non-permission-rule keys."
  - source: user settings
    scope: [rules]
    merge_strategy: shallow
    notes: "~/.claude/settings.json. Permission arrays merge with project/local rules."
  - source: user settings
    scope: [general_config]
    merge_strategy: none
    notes: "Baseline personal preferences when no narrower scope overrides them."

default_posture: "When nothing is configured, Claude Code starts in default permission mode: read-only tools (Read, Grep, Glob, LSP, etc.) run without approval, while Bash commands, file edits, Write, WebFetch, WebSearch, and other state-changing tools prompt for approval on first use."

cli_zero_permissions:
  supported: true
  invocation: 'claude --bare --setting-sources user --permission-mode dontAsk --tools "" --disallowedTools "mcp__*" --strict-mcp-config'
  mechanism: "dontAsk mode auto-denies any tool call that would otherwise prompt; --tools \"\" removes built-in tools from the session; --disallowedTools \"mcp__*\" removes configured MCP tools from the model context; --bare and --strict-mcp-config suppress most customizations and MCP discovery without mutating config."
  limitations: "Managed settings always apply. User settings still load because --setting-sources user is retained; use --settings with an explicit temporary JSON overlay if Claudine needs to neutralize a user's allow rules. Additional permissions can be added back with --tools, --allowedTools, --mcp-config, and --settings in the same invocation."

agent_permissions:
  allowed: true
  fm_properties:
    - tools
    - disallowedTools
    - permissionMode
    - mcpServers
    - hooks

yolo:
  has_interactive_yolo: true
  has_non_interactive_yolo: true
  mechanism: "--permission-mode bypassPermissions or --dangerously-skip-permissions; interactive sessions can also add it to the Shift+Tab cycle with --allow-dangerously-skip-permissions. On macOS/Linux, bypassPermissions is refused when running as root outside a recognized sandbox."

policy_engine:
  ergonomic: true
  provides_coverage: true
  gaps:
    - "Parameter-matching rules (Tool(param:value)) and the list of fields that cannot be matched this way are provider-specific."
    - "Auto mode classifier rules (autoMode.environment, allow, soft_deny, hard_deny) are prose evaluated by a model, not deterministic allow/ask/deny rules."
    - "Protected-path circuit breakers are hard-coded and apply even when static policy would predict Allow."
    - "PreToolUse hooks can block calls before permission evaluation; hook-based decisions are outside PolicyEngine's static rule model."
    - "Subagent permissionMode can be overridden at runtime when the parent uses bypassPermissions, acceptEdits, or auto mode."
    - "Managed-only administrative controls such as allowManagedPermissionRulesOnly and disableBypassPermissionsMode are policy enforcement knobs outside the user-facing permission rule surface."
    - "Sandbox filesystem/network boundaries are OS-enforced and separate from the static allow/ask/deny rule surface."
    - "Read-only Bash commands, process wrappers, compound-command splitting, and symlink path pairs are hard-coded matching behaviors."
    - "Tool visibility (--tools, subagent tools, skill visibility, plugin/customization loading, and built-in feature toggles) is adjacent to approval policy and needs first-class metadata distinct from allow/ask/deny."

permission_entities:
  - entity: tool
    native_names: ["permissions.allow", "permissions.ask", "permissions.deny", "--allowedTools", "--disallowedTools", "--tools"]
    notes: "Built-in tools such as Bash, Read, Edit, Write, WebFetch, WebSearch, Agent, Skill, etc. Bare tool names match all uses; specifiers narrow matching."
  - entity: tool_group
    native_names: ["Read, Grep, Glob, LSP", "Edit, Write, NotebookEdit"]
    notes: "Read rules apply to Read, Grep, Glob, LSP and some Bash read commands; Edit rules apply to Edit, Write, and NotebookEdit."
  - entity: command
    native_names: ["Bash", "PowerShell", "Monitor"]
    notes: "Bash and PowerShell rules match command strings with glob semantics; Monitor inherits Bash rules."
  - entity: path
    native_names: ["Read(...)", "Edit(...)", "Write(...)", "Cd(...)", "sandbox.filesystem.allowWrite", "sandbox.filesystem.denyRead"]
    notes: "Read/Edit/Write rules follow gitignore-style patterns with //, ~/, /, and relative anchors. Cd rules anchor to whole directory paths."
  - entity: workspace
    native_names: ["additionalDirectories", "--add-dir", "/add-dir"]
    notes: "Additional directories extend where Claude can read and edit, but do not load most .claude/ configuration."
  - entity: mcp_server
    native_names: ["mcpServers", "managed-mcp.json", "allowedMcpServers", "deniedMcpServers", "allowManagedMcpServersOnly", "--mcp-config", "--strict-mcp-config"]
    notes: "MCP servers can be scoped user/project/local or locked down via managed policy."
  - entity: mcp_tool
    native_names: ["mcp__<server>", "mcp__<server>__*", "mcp__<server>__<tool>"]
    notes: "MCP tools are governed by the same permission rule syntax as built-in tools."
  - entity: mcp_resource
    native_names: ["ListMcpResourcesTool", "ReadMcpResourceTool"]
    notes: "Resource discovery and reading are treated as read-only and do not prompt by default, but access depends on the connected MCP server being permitted."
  - entity: agent
    native_names: ["Agent", "Agent(Explore)", "Agent(Plan)", "Agent(<name>)"]
    notes: "The Agent tool spawns subagents. Rules can allow/deny specific agent types or the Agent tool itself."
  - entity: subagent
    native_names: ["tools", "disallowedTools", "permissionMode", "mcpServers", "hooks"]
    notes: "Subagent frontmatter can restrict tools, set a permission mode, attach MCP servers, and register scoped hooks."
  - entity: mode
    native_names: ["permissions.defaultMode", "--permission-mode", "permissionMode"]
    notes: "Permission modes set the session baseline for approvals: default, acceptEdits, plan, auto, dontAsk, bypassPermissions."
  - entity: approval_category
    native_names: ["permissions.allow", "permissions.ask", "permissions.deny"]
    notes: "Fine-grained rule decisions. Deny > ask > allow; deny from any scope beats allow from any scope."
  - entity: sandbox
    native_names: ["sandbox.enabled", "sandbox.filesystem", "sandbox.network", "sandbox.credentials", "sandbox.failIfUnavailable", "sandbox.allowUnsandboxedCommands"]
    notes: "OS-level isolation for Bash subprocesses; separate from the permission rule engine."
  - entity: hook
    native_names: ["hooks.PreToolUse", "hooks.PermissionRequest", "hooks.PermissionDenied"]
    notes: "PreToolUse hooks can deny, force a prompt, or skip a prompt before the static permission engine evaluates a call."
  - entity: extension
    native_names: ["enabledPlugins", "strictPluginOnlyCustomization", "disableSideloadFlags", "blockedMarketplaces", "strictKnownMarketplaces"]
    notes: "Plugins can bundle tools, agents, hooks, MCP servers, and output styles; managed policy can restrict plugin sources and sideloading."

approval_modes:
  - name: default
    effect: "Read-only tools run without approval; state-changing tools prompt on first use."
    interactive: true
    non_interactive: true
    aliases: ["default", "Ask before edits"]
  - name: acceptEdits
    effect: "Auto-approves file edits and common filesystem Bash commands (mkdir, touch, rm, rmdir, mv, cp, sed) inside the working directory or additionalDirectories. Other commands and out-of-scope paths still prompt."
    interactive: true
    non_interactive: true
    aliases: ["acceptEdits", "Edit automatically"]
  - name: plan
    effect: "Read-only exploration only; file edits never auto-approve. Presents a plan for approval before exiting."
    interactive: true
    non_interactive: true
    aliases: ["plan", "Plan mode"]
  - name: auto
    effect: "Routes tool calls through a background safety classifier that auto-approves routine actions and blocks risky ones. Explicit ask rules still prompt; deny rules still block."
    interactive: true
    non_interactive: true
    aliases: ["auto", "Auto mode"]
  - name: dontAsk
    effect: "Auto-denies any tool call that would otherwise prompt. Only pre-approved allow rules and read-only Bash commands execute; MCP tools marked as requiring user interaction are denied even if an allow rule matches."
    interactive: true
    non_interactive: true
    aliases: ["dontAsk", "don't ask"]
  - name: bypassPermissions
    effect: "Skips permission prompts and safety checks so tool calls execute immediately. Explicit ask rules, root/home removal circuit breakers, and MCP tools marked as requiring user interaction still prompt."
    interactive: true
    non_interactive: true
    aliases: ["bypassPermissions", "--dangerously-skip-permissions", "Bypass permissions"]

rule_model:
  decisions: ["allow", "ask", "deny"]
  syntax: "Tool or Tool(specifier). Specifiers are tool-specific: Bash(pattern), Read/Edit/Write(path-pattern), WebFetch(domain:host), Agent(name), Skill(name), mcp__server__tool. Parameter matching Tool(param:value) is supported for scalar top-level fields except fields with canonicalizing matchers (command, file_path, path, notebook_path, url)."
  precedence: "Deny rules are evaluated first, then ask rules, then allow rules. A matching deny rule always wins over a matching ask or allow rule, even if the allow rule is more specific. Deny rules from any settings scope override allow rules from any scope. The active permission mode applies after rules."
  merge_semantics: "Permission allow/ask/deny arrays merge across user, project, and local settings scopes. Other settings generally replace by precedence. Managed settings cannot be overridden by lower scopes. Sandbox filesystem arrays merge across scopes unless managed-only locks apply."
  matcher_semantics: "Bash rules use glob patterns with *; a space before * enforces a word boundary; the :* suffix is equivalent to a trailing space wildcard. Read/Edit/Write rules follow gitignore patterns with // (absolute), ~/ (home), / (project root), and relative anchors. WebFetch uses domain: prefixes with * wildcards. MCP rules use mcp__server__tool naming. Agent rules use Agent(name). Tool-name globs are allowed for deny/ask rules; allow globs are only allowed after a literal mcp__<server>__ prefix."
  default_decision: "In default mode, read-only tools are allowed and everything else asks. In dontAsk mode, the default is deny. In bypassPermissions mode, the default is allow (with circuit breakers)."

tool_visibility:
  supported: true
  mechanisms:
    - "--tools flag restricts which built-in tools are available to the session."
    - "Subagent frontmatter tools/disallowedTools restricts subagent tool surface."
    - "Skill frontmatter allowed-tools restricts skill tool surface."
    - "permissions.deny with a bare tool name removes that tool from the model's context."
    - "--disallowedTools with a bare tool name or glob removes matching tools from context."
  notes: "--tools affects only built-in tools; MCP tools are unaffected. A denied tool is hidden from the model entirely, while an allowed tool may still prompt depending on the mode and rules."

sandbox:
  supported: true
  modes: ["auto-allow", "regular-permissions"]
  backends: ["macOS Seatbelt", "Linux/WSL2 bubblewrap (with optional seccomp filter)"]
  filesystem_control: "Default write scope is the working directory plus a session temp directory. allowWrite, denyWrite, denyRead, and allowRead arrays configure boundaries. A credentials block (v2.1.187+) can deny read access to files and unset env vars. Managed-only allowManagedReadPathsOnly locks the allowRead list to managed settings."
  network_control: "No domains are pre-allowed. allowedDomains and deniedDomains configure network access. A custom proxy can be configured via httpProxyPort/socksProxyPort. The built-in proxy does not terminate TLS, so domain fronting is possible if broad domains are allowed. allowManagedDomainsOnly locks the allowlist to managed settings."
  notes: "Sandboxing applies only to Bash subprocesses. Native Windows is not supported (use WSL2). If dependencies are missing, Claude Code warns and falls back to unsandboxed execution unless failIfUnavailable is true. allowUnsandboxedCommands provides an escape hatch that retries blocked commands outside the sandbox. Auto-allow mode bypasses the bare Bash ask rule for sandboxed commands but still honors explicit deny and content-scoped ask rules."

trust_and_admin:
  folder_trust: "First-time launches in a project directory prompt a workspace trust dialog. Untrusted folders ignore project CLAUDE.md, .mcp.json approvals (unless approved by user/managed/local settings), and checked-in enableAllProjectMcpServers. Trust is saved per directory unless Claude Code is started directly from the home directory, where acceptance is session-only. Trust verification is skipped in non-interactive -p mode."
  managed_policy: "Managed settings are delivered via server (claude.ai admin console), MDM plist, Windows registry, or system managed-settings.json. They occupy the highest precedence tier and cannot be overridden by user/project/local config or CLI flags. A policyHelper can preempt all other managed sources. Managed-only keys include allowManagedPermissionRulesOnly, allowManagedMcpServersOnly, allowManagedHooksOnly, disableBypassPermissionsMode, disableSideloadFlags, sandbox.filesystem.allowManagedReadPathsOnly, sandbox.network.allowManagedDomainsOnly, and others."
  safe_mode: "--safe-mode or CLAUDE_CODE_SAFE_MODE=1 disables CLAUDE.md, skills, plugins, hooks, MCP servers, custom commands/agents, output styles, workflows, themes, keybindings, status line, file-suggestion commands, LSP, and auto-memory. Built-in tools and permissions continue to work normally."
  notes: "--bare is similar but additionally limits the default tool set to Bash, Read, and Edit and disables OAuth/keychain reads. Safe mode and bare mode are session-scoped."

mcp_permissions:
  supported: true
  server_filters:
    - "managed-mcp.json delivers a fixed server set with exclusive control."
    - "allowedMcpServers / deniedMcpServers filter servers by serverUrl, serverCommand, or serverName."
    - "allowManagedMcpServersOnly restricts the allowlist to managed settings."
    - "--strict-mcp-config ignores all MCP config except --mcp-config."
    - "disableClaudeAiConnectors or ENABLE_CLAUDEAI_MCP_SERVERS=false disables claude.ai connectors."
    - "strictPluginOnlyCustomization can block plugin-provided MCP servers."
  tool_filters:
    - "Permission rules mcp__<server>, mcp__<server>__*, and mcp__<server>__<tool> allow/ask/deny specific tools."
    - "--allowedTools and --disallowedTools accept the same MCP patterns."
  trust_model: "Project-scoped .mcp.json servers require user approval via a trust dialog; untrusted repos ignore project .mcp.json approvals. OAuth tokens are stored per user. Remote servers may require re-authentication. claude.ai connectors are provisioned organization-wide."
  notes: "MCP tools run outside the Bash sandbox. stdio servers are subprocesses and can be scrubbed via CLAUDE_CODE_MCP_ALLOWLIST_ENV and CLAUDE_CODE_SUBPROCESS_ENV_SCRUB. In non-interactive mode, servers needing auth are reported as unavailable rather than hanging."

headless_behavior: "In non-interactive -p mode, interactive permission prompts cannot be shown. Use --allowedTools, --permission-mode dontAsk, or --permission-prompt-tool to avoid hangs. Auto mode repeated classifier blocks abort the session. MCP servers requiring OAuth are reported unavailable. Security approval dialogs and workspace trust dialogs are skipped in -p mode, so project MCP servers load only if already approved or allowed by user/managed/local settings."

approval_persistence: "Bash 'Yes, don't ask again' approvals persist permanently per project directory and command pattern. File edit approvals persist until the end of the session. Protected-path write prompts can be approved for the current session. Sandbox allowed domains persist for the rest of the session as of v2.1.191."

protected_paths:
  - ".git"
  - ".config/git"
  - ".vscode"
  - ".idea"
  - ".husky"
  - ".cargo"
  - ".devcontainer"
  - ".yarn"
  - ".mvn"
  - ".claude (except .claude/worktrees)"
  - ".gitconfig, .gitmodules"
  - ".bashrc, .bash_profile, .bash_login, .bash_aliases, .bash_logout, .zshrc, .zprofile, .zshenv, .zlogin, .zlogout, .profile, .envrc"
  - ".npmrc, .yarnrc, .yarnrc.yml, .pnp.cjs, .pnp.loader.mjs, .pnpmfile.cjs, bunfig.toml, .bunfig.toml"
  - ".bazelrc, .bazelversion, .bazeliskrc"
  - ".pre-commit-config.yaml, lefthook.yml, lefthook.yaml, .lefthook.yml, .lefthook.yaml"
  - "gradle-wrapper.properties, maven-wrapper.properties"
  - ".devcontainer.json"
  - ".ripgreprc, pyrightconfig.json"
  - ".mcp.json, .claude.json"

security_posture: "Claude Code's default security is a client-side static policy engine with advisory prompts, not an OS-enforced sandbox. An optional OS-enforced sandbox (Seatbelt on macOS, bubblewrap on Linux/WSL2) can restrict Bash subprocess filesystem and network access. Auto mode adds a model-based classifier. Managed settings provide administrative policy but are still client-side controls. Defense-in-depth requires combining permission rules, sandboxing, and managed policy."

changes:
  - "Documented expanded permission rule syntax: parameter matching Tool(param:value), :* suffix, tool-name globs, and fields that cannot be parameter-matched."
  - "Added the auto mode classifier details, including environment configuration, classifyAllShell, v2.1.195 expanded block/allow categories, and project/local settings ignoring defaultMode auto."
  - "Added sandbox coverage: auto-allow vs regular-permissions modes, credentials block, managed-only read/domain locks, OS backends, and the allowUnsandboxedCommands escape hatch."
  - "Expanded MCP policy coverage: managed-mcp.json exclusive control, allowedMcpServers/deniedMcpServers matching semantics, allowManagedMcpServersOnly, project .mcp.json trust gating, and claude.ai connector controls."
  - "Added trust and administrative controls section covering workspace trust, managed settings tiers, managed-only keys, safe mode, and --bare."
  - "Added new CLI flags relevant to config loading: --setting-sources and --settings."
  - "Updated env_vars list with additional permission/security variables discovered in current docs."
  - "Added all new schema-required frontmatter fields (cli_zero_permissions, permission_entities, rule_model, tool_visibility, sandbox, trust_and_admin, mcp_permissions, headless_behavior, approval_persistence, protected_paths, security_posture)."

requires_claudine_update: true
reason: "The research surfaced several permission/security concepts that Claudine's PolicyEngine and provider catalog do not yet model: parameter-matching rules, sandbox filesystem/network boundaries and auto-allow mode, MCP server filters and trust gating, PreToolUse hooks, subagent mode override behavior, managed-only locks, and tool-name glob semantics. Capturing these accurately will require code/metadata updates beyond the research document."
---

# Claude Code Permissions

## Introduction to Claude Code Permissions

Claude Code uses a tiered permission system that balances power and safety. Read-only actions such as file reads, Grep, Glob, and LSP are allowed by default. Actions that can change state such as Bash commands, file edits, Write, WebFetch, and WebSearch require approval unless pre-approved by a permission rule or permission mode.

Permissions can be defined in three ways:

1. **Configuration files** in `settings.json` at user, project, local, or managed scope.
2. **CLI flags** passed at startup such as `--permission-mode`, `--allowedTools`, and `--disallowedTools`.
3. **In-session controls** such as `/permissions`, `/config`, and the `Shift+Tab` mode selector.

Local observation on 2026-07-03: this host has a `~/.claude/settings.json` file with `permissions.allow`, `permissions.deny`, `enabledPlugins`, `extraKnownMarketplaces`, `skipDangerousModePermissionPrompt`, and model/effort settings. The active repo has `.claude/settings.local.json`, but it was empty for scalar values relevant to this research. `~/.claude.json` exists and contains Claude Code state/cache/project keys, which matches the documented split where user/local MCP state is outside `settings.json`.

The permission system evaluates rules in this order: deny rules first, then ask rules, then allow rules, and finally the active permission mode. A matching deny rule always wins over an allow rule, and rule specificity does not change that order. A broad deny rule like `Bash(aws *)` blocks every matching call, including calls that also match a narrower allow rule like `Bash(aws s3 ls)`.

### Permission modes

Claude Code supports six permission modes. The mode acts as a baseline; allow/ask/deny rules can refine it.

| Mode | What runs without asking | Best for |
| :----- | :----- | :----- |
| `default` | Read-only tools only | Getting started, sensitive work |
| `acceptEdits` | Reads, file edits, and common filesystem commands (`mkdir`, `touch`, `mv`, `cp`, `rm`, `rmdir`, `sed`) in the working directory | Iterating on code you review after the fact |
| `plan` | Read-only tools only; file edits never auto-approve | Exploring before changing code |
| `auto` | Everything, routed through a background safety classifier | Long tasks with fewer prompts |
| `dontAsk` | Only pre-approved tools; everything else is denied | Locked-down CI and scripts |
| `bypassPermissions` | Everything (except explicit ask rules and root/home removal circuit breakers) | Isolated containers and VMs only |

### Permission rule syntax

Permission rules follow the form `Tool` or `Tool(specifier)`.

| Rule | Effect |
| :----- | :----- |
| `Bash` | Matches all Bash commands |
| `Bash(npm run *)` | Matches commands starting with `npm run ` |
| `Read(./.env)` | Matches reading `.env` in the current directory |
| `Edit(/src/**/*.ts)` | Matches edits under `<repo>/src/` |
| `WebFetch(domain:example.com)` | Matches fetches to `example.com` |
| `Agent(Explore)` | Matches the Explore subagent |
| `mcp__puppeteer__*` | Matches every tool from the `puppeteer` MCP server |

Rules live in the `permissions` object of `settings.json` under `allow`, `ask`, and `deny` arrays. Deny rules can use bare tool names to remove the tool from Claude's context, or scoped rules to block matching calls.

### CLI parameters and precedence

The permission-related CLI parameters are listed in the frontmatter. In summary:

- `--permission-mode <mode>` sets the session's permission mode.
- `--allowedTools <rules>` adds allow rules.
- `--disallowedTools <rules>` adds deny rules.
- `--dangerously-skip-permissions` and `--allow-dangerously-skip-permissions` control bypassPermissions mode.
- `--add-dir`, `--tools`, `--mcp-config`, `--strict-mcp-config`, and `--permission-prompt-tool` adjust scope, tool availability, MCP loading, and programmatic approval.

Precedence is documented in the frontmatter. The key points are:

- Managed settings cannot be overridden by any other source.
- CLI flags are temporary session overrides.
- Local project settings override project and user settings.
- Project settings override user settings.
- For permission rules specifically, a deny rule from any scope blocks the tool even if another scope allows it.

### Permission policy vs tool visibility

Claude Code separates **which tools are visible to the model** from **which visible tools are pre-approved**.

- **Approval policy** (`permissions.allow`/`ask`/`deny`, `--allowedTools`, `--disallowedTools`, permission modes) decides whether a tool call runs and whether it prompts.
- **Tool visibility** (`--tools`, subagent `tools`/`disallowedTools`, skill `allowed-tools`, bare-name deny rules) decides which tools appear in the model's context at all. A tool removed by a bare deny rule never appears in the prompt, so the model cannot choose to invoke it.

For example, `--tools "Bash,Read,Edit"` hides every built-in tool except Bash, Read, and Edit, while `--allowedTools "Bash(npm test)"` still leaves Bash visible but only auto-approves `npm test`. MCP tools are not constrained by `--tools`; use `mcp__*` deny rules to hide them.

## Permissions Use Cases

### Default

If no environment variable, config file, or CLI switch configures permissions, Claude Code starts in `default` mode with the posture described in the frontmatter: read-only tools are free, and state-changing tools prompt on first use.

A PolicyEngine description of the default posture would be:

- `can_read(path)` → Allow for paths in the working directory and additional directories.
- `can_write(path)` → Ask for paths in the working directory; Deny for paths outside it until approved.
- `can_execute(command)` → Ask for Bash and PowerShell commands.
- `can_access_domain(domain)` → Ask for WebFetch/WebSearch.
- `can_use_mcp_server(server)` / `can_use_mcp_tool(server, tool)` → Ask until approved or denied.
- `can_spawn_subagent(agent)` → Allow to spawn, but the subagent's own tool calls are checked independently.

This use case is ergonomic in PolicyEngine because the engine already models read, write, execute, network, MCP, and agent axes. No changes are required for PolicyEngine to describe it. The main limitation is that PolicyEngine returns static snapshots; the interactive approval prompt itself is a runtime UI concern, not a policy fact.

### Whitelisting

To start with no permissions and require every needed permission to be asked for or explicitly declared, use `dontAsk` mode combined with `permissions.allow` rules.

In `settings.json`:

```json
{
  "permissions": {
    "defaultMode": "dontAsk",
    "allow": ["Read", "Grep", "Glob"]
  }
}
```

With this configuration, only Read, Grep, and Glob run without a prompt. Bash, Edit, Write, WebFetch, and every other tool are denied unless you add them to `allow` or pass `--allowedTools` at startup.

CLI examples:

```bash
# Run tests in a locked-down CI invocation
claude -p --permission-mode dontAsk --allowedTools "Bash(npm test),Read" "run the test suite"

# Allow only read-only tools for a codebase exploration
claude --permission-mode dontAsk --allowedTools "Read,Grep,Glob" "explain the auth module"

# Add a temporary domain allowlist for one session
claude --permission-mode dontAsk   --allowedTools "Read,Grep,WebFetch(domain:docs.rs)" "research Rust docs"
```

In interactive sessions, you can still use `/permissions` to add allow rules on the fly, but `dontAsk` prevents prompts; it denies anything not pre-approved.

The best **CLI-only, session-scoped** way to start from zero tools is documented in the frontmatter's `cli_zero_permissions` field:

```bash
claude --bare --setting-sources user --permission-mode dontAsk --tools "" --disallowedTools "mcp__*" --strict-mcp-config
```

This disables all built-in tools, denies configured MCP tools, and suppresses most customization and MCP discovery without mutating the user's provider config. You can then add back only what is needed with `--tools`, `--allowedTools`, `--mcp-config`, and a temporary `--settings` JSON overlay. Managed settings still apply, and user settings still load in the invocation above; a Claudine wrapper should supply a temporary settings overlay if it needs to neutralize user-scope allow rules.

PolicyEngine can describe this use case by setting `SetApprovalMode(dontAsk)` and adding allow rules for the approved tool surface. It is ergonomic and provides coverage for the deterministic part of the policy. The gap is that PolicyEngine cannot force an interactive user to be asked; it can only report that the effective policy would deny the call. The actual ask-or-deny behavior is a runtime decision made by Claude Code's UI layer.

### YOLO

In Claude Code, YOLO mode is called `bypassPermissions`. A session can be put into this mode by:

- Starting with `--permission-mode bypassPermissions`.
- Starting with `--dangerously-skip-permissions` (equivalent to the above).
- Starting with `--allow-dangerously-skip-permissions`, which adds `bypassPermissions` to the interactive `Shift+Tab` cycle without activating it immediately.
- Setting `permissions.defaultMode` to `bypassPermissions` in a settings file.

Availability:

- **Interactive sessions**: yes, when started with one of the enabling flags or when the default mode is set to `bypassPermissions`.
- **Non-interactive sessions**: yes, `claude -p --dangerously-skip-permissions` works.
- **Root/sudo on macOS and Linux**: no. Claude Code refuses to start in `bypassPermissions` mode as root or under sudo, with the error `--dangerously-skip-permissions cannot be used with root/sudo privileges for security reasons`. The check is skipped inside a recognized sandbox or dev container.

When in `bypassPermissions` mode:

- **Allowed**: almost all tool calls execute without prompting, including file edits, Bash commands, WebFetch, WebSearch, MCP tool calls, and subagent spawns.
- **Still prompted**: explicit `ask` rules in configuration still force a prompt; removals targeting the filesystem root or home directory (`rm -rf /`, `rm -rf ~`) still prompt as a circuit breaker.
- **Not allowed**: it cannot override managed settings that disable the mode via `permissions.disableBypassPermissionsMode`.

### Root User

When Claude Code is started as root or under sudo on macOS or Linux, it behaves differently with regard to `bypassPermissions`:

- `bypassPermissions` mode is refused at startup unless the process is inside a recognized sandbox or dev container.
- Other permission modes (`default`, `acceptEdits`, `plan`, `auto`, `dontAsk`) work normally for root.
- YOLO/bypassPermissions is therefore not available to a root session outside a sandbox.

This is a hardcoded safety check, not a configurable policy rule.

### Configuring the Default

Default permissions are configured through `settings.json` files at three main scopes:

- **User scope**: `~/.claude/settings.json` applies across all projects.
- **Repo/project scope**: `.claude/settings.json` applies to everyone working in the repository and can be checked into version control.
- **Local scope**: `.claude/settings.local.json` applies only to you in this repository and is typically gitignored.

For the schema's `config_files` field, user scope is `~/.claude/settings.json` and repo scope is `.claude/settings.json`. Local overrides live in `.claude/settings.local.json`.

Examples that illustrate the grammar:

```json
// ~/.claude/settings.json — user-wide defaults
{
  "permissions": {
    "defaultMode": "acceptEdits",
    "allow": [
      "Bash(npm run *)",
      "Bash(git status *)",
      "WebFetch(domain:docs.rs)"
    ],
    "deny": [
      "Bash(curl *)",
      "Bash(wget *)",
      "Read(~/.ssh/**)"
    ]
  }
}
```

```json
// .claude/settings.json — repo-shared defaults
{
  "permissions": {
    "defaultMode": "default",
    "allow": [
      "Bash(npm run lint)",
      "Bash(npm run test *)"
    ],
    "deny": [
      "Read(./.env)",
      "Read(./secrets/**)"
    ]
  }
}
```

```json
// .claude/settings.local.json — personal repo overrides
{
  "permissions": {
    "allow": [
      "Bash(docker *)",
      "WebFetch(domain:localhost:*)"
    ]
  }
}
```

The `permissions` object also supports:

- `additionalDirectories` — directories treated like the working directory for read/edit permissions.
- `disableBypassPermissionsMode` — set to `"disable"` to prevent use of `bypassPermissions` mode.
- `disableAutoMode` — set to `"disable"` to prevent use of `auto` mode.
- `autoMode` — prose classifier rules (`environment`, `allow`, `soft_deny`, `hard_deny`) for auto mode.

### Extending the Base

Default permissions can be set at user scope and then narrowed or extended by narrower scopes.

**Example 1: user allows, repo denies.**

User `~/.claude/settings.json`:

```json
{
  "permissions": {
    "allow": ["Bash(curl *)"]
  }
}
```

Repo `.claude/settings.json`:

```json
{
  "permissions": {
    "deny": ["Bash(curl *)"]
  }
}
```

Result: `curl` is blocked in the repository because deny rules from any scope override allow rules.

**Example 2: user default mode, CLI override.**

User `~/.claude/settings.json`:

```json
{
  "permissions": {
    "defaultMode": "acceptEdits"
  }
}
```

CLI:

```bash
claude --permission-mode plan
```

Result: the session starts in `plan` mode. CLI flags override settings.

**Example 3: project allowlist, local addition.**

Repo `.claude/settings.json`:

```json
{
  "permissions": {
    "defaultMode": "dontAsk",
    "allow": ["Read", "Grep", "Bash(npm test)"]
  }
}
```

Local `.claude/settings.local.json`:

```json
{
  "permissions": {
    "allow": ["Bash(npm run build)"]
  }
}
```

Result: in this repository, `npm test` and `npm run build` are both allowed, along with Read and Grep, because allow rules merge across scopes.

## Tools and Permissions

Claude Code provides the following built-in tools. The "Permission Required" column indicates whether the tool prompts by default in `default` mode.

| Tool | Permission Required | Notes |
| :----- | :----- | :----- |
| `Agent` | No | Spawns subagents; subagent tool calls are checked independently. |
| `Artifact` | Yes | Publishes shareable artifacts. |
| `AskUserQuestion` | No | Gathers requirements. |
| `Bash` | Yes | Executes shell commands. Read-only commands such as `ls`, `cat`, `git status` run without prompting. |
| `CronCreate` / `CronDelete` / `CronList` | No | Session scheduling. |
| `Edit` | Yes | Targeted file edits. |
| `EnterPlanMode` | No | Switches to plan mode. |
| `EnterWorktree` | No | Creates/switches git worktrees. |
| `ExitPlanMode` | Yes | Presents plan for approval. |
| `ExitWorktree` | No | Returns to original directory. |
| `Glob` / `Grep` / `LSP` | No | File/content search and code intelligence. |
| `ListMcpResourcesTool` / `ReadMcpResourceTool` | No | MCP resource discovery/reading. |
| `Monitor` | Yes | Background watches; Bash rules apply to command sources. |
| `NotebookEdit` | Yes | Jupyter notebook edits. |
| `PowerShell` | Yes | PowerShell commands on Windows. |
| `PushNotification` | No | Desktop/phone notifications. |
| `Read` | No | Reads file contents. |
| `RemoteTrigger` | No | Manages claude.ai Routines. |
| `ReportFindings` | No | Code-review findings. |
| `ScheduleWakeup` | No | Self-paced `/loop` scheduling. |
| `SendMessage` | No | Agent-team/subagent messaging. |
| `SendUserFile` | No | Sends files to your device. |
| `ShareOnboardingGuide` | Yes | Uploads onboarding guide. |
| `Skill` | Yes | Executes a skill. |
| `TaskCreate` / `TaskGet` / `TaskList` / `TaskOutput` / `TaskStop` / `TaskUpdate` | No | Task list management. |
| `TodoWrite` | No | Session checklist (disabled by default in v2.1.142+). |
| `ToolSearch` / `WaitForMcpServers` | No | MCP server discovery/waiting. |
| `WebFetch` / `WebSearch` | Yes | Network requests. |
| `Workflow` | Yes | Dynamic multi-subagent workflows. |
| `Write` | Yes | Creates or overwrites files. |

Permissions map to tool calls via the rule syntax described above. An `Edit(...)` allow rule also grants read access to the same path. Bash permission rules support glob patterns and recognize common read-only commands. Read/Edit rules follow gitignore-style patterns with `//`, `~/`, `/`, and relative anchors. WebFetch rules use `domain:` prefixes.

### Rule grammar details

- **Deny/ask/allow precedence**: rules are evaluated in the order deny, ask, allow. The first match wins. A broad deny rule blocks a narrower allow rule.
- **Bare tool names**: `Bash` or `Bash(*)` as a deny rule removes Bash from the model's context entirely.
- **Scoped rules**: `Bash(rm *)` leaves Bash available but blocks matching calls.
- **Parameter matching**: `Tool(param:value)` matches a scalar top-level input parameter. `*` is supported inside the value. Fields that already have canonicalizing matchers (`command` for Bash/PowerShell, `file_path` for Read/Edit/Write, `path` for Grep/Glob, `notebook_path` for NotebookEdit, `url` for WebFetch) cannot be matched this way.
- **Wildcards**: Bash patterns use `*` with word-boundary semantics when preceded by a space; `Bash(ls:*)` is equivalent to `Bash(ls *)`. Tool-name globs work in deny/ask rules (`"*"`, `"mcp__*"`); allow globs are only allowed after a literal `mcp__<server>__` prefix.
- **Compound commands**: Claude Code recognizes shell operators (`&&`, `||`, `;`, `|`, `|&`, `&`, newlines) and checks each subcommand independently. Approving a compound command with "Yes, don't ask again" saves a separate rule per subcommand.
- **Process wrappers**: `timeout`, `time`, `nice`, `nohup`, `stdbuf`, and bare `xargs` are stripped before matching. Exec wrappers such as `watch`, `setsid`, `ionice`, and `flock` always prompt and cannot be auto-approved by prefix rules.
- **Read-only Bash commands**: a built-in set including `ls`, `cat`, `echo`, `pwd`, `head`, `tail`, `grep`, `find`, `wc`, `which`, `diff`, `stat`, `du`, `cd`, and read-only `git` commands run without prompting in every mode.
- **PowerShell**: rules are case-insensitive and aliases are canonicalized to cmdlet names.
- **Symlinks**: allow rules require both the symlink and its target to match; deny rules apply if either matches.
- **Cd rules**: `Cd` is not a model-invocable tool; rules control the `/cd` command. A bare `Cd` deny disables `/cd`; any allow rule switches `/cd` to allowlist mode.

## Sandboxing, Trust, and Administrative Controls

### Sandboxing

Claude Code's sandbox is a separate layer from permission modes and rules. It provides OS-level filesystem and network isolation for Bash subprocesses.

- **Backends**: macOS uses Seatbelt; Linux and WSL2 use bubblewrap with an optional seccomp filter.
- **Modes**:
  - **Auto-allow**: sandboxed Bash commands run without prompting; the sandbox boundary substitutes for the bare `Bash` ask rule.
  - **Regular permissions**: sandboxed commands still go through the regular permission flow.
- **Filesystem**: by default, sandboxed commands can write only to the working directory and a session temp directory. Use `sandbox.filesystem.allowWrite`, `denyWrite`, `denyRead`, and `allowRead` to customize paths.
- **Network**: no domains are pre-allowed. Use `sandbox.network.allowedDomains` and `deniedDomains` to configure access. A custom proxy can be configured for TLS inspection.
- **Credentials**: `sandbox.credentials` (v2.1.187+) can deny reads of credential files and unset secret environment variables for sandboxed commands.
- **Escape hatches**: `allowUnsandboxedCommands` lets Claude retry a blocked command outside the sandbox; set it to `false` for strict sandboxing. `failIfUnavailable` blocks startup if sandbox dependencies are missing.
- **Scope**: sandboxing applies only to Bash subprocesses. Built-in file tools, MCP tools, and computer use run outside this boundary.

Permissions and sandboxing are complementary:

- Permission rules block Claude from attempting restricted actions.
- Sandbox restrictions prevent Bash commands from reaching resources outside defined boundaries, even if a prompt injection bypasses Claude's decision-making.

### Trust and administrative controls

**Folder/project trust**: first-time launches in a project directory prompt a workspace trust dialog. Untrusted folders ignore project `CLAUDE.md`, `.mcp.json` approvals (unless approved by user/managed/local settings), and checked-in `enableAllProjectMcpServers`. Trust is saved per directory unless Claude Code is started directly from the home directory, where acceptance is session-only. Trust verification is skipped in non-interactive `-p` mode.

**Managed/admin policy**: managed settings are delivered via server (claude.ai admin console), MDM plist, Windows registry, or system `managed-settings.json`. They occupy the highest precedence tier and cannot be overridden by user/project/local config or CLI flags. Key managed-only controls include:

- `allowManagedPermissionRulesOnly` — prevents user/project permission rules.
- `allowManagedMcpServersOnly` — locks the MCP server allowlist to managed settings.
- `allowManagedHooksOnly` — blocks user/project/plugin hooks except those force-enabled in managed settings.
- `disableBypassPermissionsMode` — disables YOLO mode.
- `disableSideloadFlags` — rejects `--plugin-dir`, `--plugin-url`, `--agents`, and `--mcp-config` at startup.
- `sandbox.filesystem.allowManagedReadPathsOnly` and `sandbox.network.allowManagedDomainsOnly` — prevent widening sandbox policy from lower scopes.
- `strictPluginOnlyCustomization` — restricts skills, agents, hooks, and MCP servers to plugins or managed settings.

**Safe mode**: `--safe-mode` or `CLAUDE_CODE_SAFE_MODE=1` disables CLAUDE.md, skills, plugins, hooks, MCP servers, custom commands/agents, output styles, workflows, themes, keybindings, status line, file-suggestion commands, LSP, and auto-memory. Built-in tools and permissions continue to work normally.

### Protected paths

Writes to a small set of paths are never auto-approved in any mode except `bypassPermissions`:

- Directories: `.git`, `.config/git`, `.vscode`, `.idea`, `.husky`, `.cargo`, `.devcontainer`, `.yarn`, `.mvn`, `.claude` (except `.claude/worktrees`).
- Files: `.gitconfig`, `.gitmodules`, shell config files (`.bashrc`, `.zshrc`, `.profile`, `.envrc`, etc.), package-manager config files (`.npmrc`, `.yarnrc`, `bunfig.toml`, etc.), `.bazelrc`, `.bazelversion`, `.bazeliskrc`, pre-commit config files, Gradle/Maven wrapper properties, `.devcontainer.json`, `.ripgreprc`, `pyrightconfig.json`, `.mcp.json`, `.claude.json`.

Allow rules do not pre-approve protected-path writes. In modes that prompt, the prompt offers **Yes, and allow Claude to edit its own settings for this session**, which approves later protected-path writes in that session.

## MCP and Permissions

MCP servers extend Claude Code with external tools. Once connected, their tools appear as `mcp__<server>__<tool>` and are governed by the same permission system as built-in tools.

Permission rules for MCP:

- `mcp__<server>` matches any tool from that server.
- `mcp__<server>__*` matches every tool from that server using wildcard syntax.
- `mcp__<server>__<tool>` matches a specific tool.
- `mcp__*` as a deny rule removes every MCP tool from Claude's context.

Administrators can make MCP safer through several mechanisms:

- **Managed MCP configuration**: deploy `managed-mcp.json` to define a fixed server set or disable MCP entirely.
- **Allowlists/denylists**: use `allowedMcpServers` and `deniedMcpServers` in managed settings, matching by `serverUrl`, `serverCommand`, or `serverName`.
- **Strict config**: use `--strict-mcp-config` to ignore all MCP configuration except what is passed via `--mcp-config`.
- **Environment scrubbing**: set `CLAUDE_CODE_MCP_ALLOWLIST_ENV=1` to limit the environment passed to stdio MCP servers, and `CLAUDE_CODE_SUBPROCESS_ENV_SCRUB=1` to strip credentials from subprocess environments.
- **Disable claude.ai connectors**: set `disableClaudeAiConnectors` to `true` or `ENABLE_CLAUDEAI_MCP_SERVERS=false`.
- **Tool-level permission rules**: add `deny` rules such as `mcp__filesystem__write_file` or `mcp__github__create_issue` to block specific high-risk operations while keeping the server connected.

When a configured MCP server is blocked by policy, it silently disappears from `/mcp` and `claude mcp list`; users see no warning that policy is the cause. In non-interactive mode with tool search enabled, Claude Code tells Claude that the server's tools are unavailable rather than pretending the server is not configured.

MCP tools run outside the Bash sandbox. Remote MCP servers make network requests from outside Claude Code's process, and stdio MCP servers run as local subprocesses. Use the server filters and tool-level deny rules to constrain them.

## Non-Interactive Behavior

In non-interactive `-p` mode, Claude Code cannot show interactive permission prompts. Use one of these strategies:

- Pass `--allowedTools` with the rules you want auto-approved.
- Start in `--permission-mode dontAsk` and pre-define all needed permissions.
- Use `--permission-prompt-tool <tool>` to route permission prompts to an MCP tool for programmatic approval.
- Use `--bare` to skip auto-discovery of project/user customizations and make runs reproducible.

Auto mode in `-p` works, but if the classifier blocks an action repeatedly the session aborts because there is no user to prompt. MCP servers requiring OAuth authentication are reported as unavailable in `-p` rather than hanging. Security approval dialogs and workspace trust dialogs are skipped in `-p` mode, so project-local `.mcp.json` servers load only if already approved or allowed by user/managed/local settings.

## Sources

- [Claude Code overview](https://code.claude.com/docs/en/overview)
- [Configure permissions](https://code.claude.com/docs/en/permissions)
- [Choose a permission mode](https://code.claude.com/docs/en/permission-modes)
- [Configure the sandboxed Bash tool](https://code.claude.com/docs/en/sandboxing)
- [Claude Code settings](https://code.claude.com/docs/en/settings)
- [CLI reference](https://code.claude.com/docs/en/cli-reference)
- [Environment variables](https://code.claude.com/docs/en/env-vars)
- [Tools reference](https://code.claude.com/docs/en/tools-reference)
- [Create custom subagents](https://code.claude.com/docs/en/sub-agents)
- [Connect Claude Code to tools via MCP](https://code.claude.com/docs/en/mcp)
- [Control MCP server access for your organization](https://code.claude.com/docs/en/managed-mcp)
- [Configure server-managed settings](https://code.claude.com/docs/en/server-managed-settings)
- [Configure auto mode](https://code.claude.com/docs/en/auto-mode-config)
- [Run Claude Code programmatically](https://code.claude.com/docs/en/headless)
- [Hooks reference](https://code.claude.com/docs/en/hooks)
- [Security](https://code.claude.com/docs/en/security)

## Changelog

- 2026-07-03: Refreshed metadata for Codex/default run, split `config_files` into macOS/Linux/Windows records to satisfy `_schema.yaml`, verified the installed local CLI as Claude Code 2.1.200, inspected local `~/.claude` and repo `.claude` config shapes, added session-scoped `--agent`, `--agents`, channel, and worktree security-control flags, strengthened the CLI-only zero-permissions invocation, and updated `dontAsk`/`bypassPermissions` notes for MCP tools requiring user interaction.
- 2026-07-02: Refreshed research against current Claude Code documentation. Added comprehensive coverage of sandboxing, trust/admin controls, MCP permissions, non-interactive behavior, protected paths, and expanded rule grammar including parameter matching and tool-name globs. Updated frontmatter to the full schema contract and flagged Claudine updates as required.
