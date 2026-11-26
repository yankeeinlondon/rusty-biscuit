---
$schema: ./_schema.yaml
created: 2026-07-01
last_updated: 2026-07-03
agent: codex
model: default

cli_params:
  - param: approval-mode
    style: switch
    description: Sets the session approval mode. Values are default, auto_edit, yolo, and plan. Overrides general.defaultApprovalMode for the session; yolo is CLI-only and cannot be stored as the default approval mode.
    example: gemini --approval-mode plan
    example_description: Starts an interactive session in read-only Plan Mode.
  - param: yolo
    style: switch
    description: Deprecated alias for --approval-mode=yolo. Auto-approves nearly all tool actions unless disabled by security.disableYoloMode or secure mode.
    example: gemini -y -p "update the changelog"
    example_description: Runs a non-interactive prompt with YOLO approval mode.
  - param: sandbox
    style: switch
    description: Enables sandboxing for this session. The CLI flag is boolean; provider selection can come from GEMINI_SANDBOX or settings.
    example: gemini --sandbox -p "run npm test"
    example_description: Runs a headless prompt with sandboxing enabled.
  - param: skip-trust
    style: switch
    description: Trusts the current workspace for this session by setting GEMINI_CLI_TRUST_WORKSPACE=true internally, bypassing the folder trust dialog.
    example: gemini --skip-trust -p "summarize this repository"
    example_description: Allows headless execution in a workspace that would otherwise require a trust prompt.
  - param: policy
    style: switch
    description: Loads additional User-tier policy TOML files or directories. Takes comma-separated values or repeated flags.
    example: gemini --policy ./gemini-policy.d
    example_description: Adds supplemental per-session User-tier rules without editing ~/.gemini/policies.
  - param: admin-policy
    style: switch
    description: Loads additional Admin-tier policy TOML files or directories. Ignored when standard system policy directories already contain TOML files.
    example: gemini --admin-policy /etc/gemini-cli/extra-policies
    example_description: Adds supplemental Admin-tier rules for the session or wrapper environment.
  - param: allowed-mcp-server-names
    style: switch
    description: Restricts configured MCP servers to the named server list for this session. Accepts comma-separated values or repeated flags.
    example: gemini --allowed-mcp-server-names github,docs
    example_description: Connects only the github and docs MCP servers.
  - param: allowed-tools
    style: switch
    description: Deprecated tool allowlist. Accepts comma-separated values or repeated flags and creates temporary allow rules for matching tools.
    example: gemini --allowed-tools read_file,glob
    example_description: Legacy way to auto-allow selected tools without confirmation.
  - param: exclude-tools
    style: switch
    description: Temporary tool blocklist exposed in current source. Accepts comma-separated values or repeated flags and creates temporary block rules; it is not listed in the packaged CLI reference table.
    example: gemini --exclude-tools run_shell_command,write_file
    example_description: Blocks shell execution and file writes for the session through the legacy tool-filter path.
  - param: include-directories
    style: switch
    description: Adds additional directories to the workspace. Accepts comma-separated values or repeated flags and can expand file-tool and sandbox scope.
    example: gemini --include-directories ../shared,../docs
    example_description: Makes sibling shared and docs directories available to the session.
  - param: extensions
    style: switch
    description: Selects which extensions to load for the session. Accepts comma-separated values or repeated flags; if omitted, all enabled extensions are used.
    example: gemini --extensions corp-tools
    example_description: Loads only the corp-tools extension, limiting extension-provided tools, MCP servers, hooks, and policies.
  - param: list-extensions
    style: switch
    description: Lists available extensions and exits, useful for auditing extension surfaces before launch.
    example: gemini --list-extensions
    example_description: Prints extension inventory without starting a session.
  - param: prompt
    style: switch
    description: Runs in non-interactive headless mode with the given prompt. In this mode ask_user decisions are treated as deny and approval prompts cannot be answered.
    example: gemini -p --approval-mode plan "summarize README.md"
    example_description: Runs a headless read-only prompt.
  - param: prompt-interactive
    style: switch
    description: Executes the provided prompt and continues in interactive mode, so approval and trust dialogs can still be displayed.
    example: gemini -i "inspect the build scripts"
    example_description: Starts interactively after the initial prompt.
  - param: acp
    style: switch
    description: Starts the agent in ACP mode. This changes the client protocol surface and should be treated as an adjacent control surface for wrappers.
    example: gemini --acp
    example_description: Starts Gemini CLI in ACP mode.
  - param: experimental-acp
    style: switch
    description: Deprecated alias for --acp.
    example: gemini --experimental-acp
    example_description: Starts the deprecated ACP entry path.
  - param: output-format
    style: switch
    description: Sets output format to text, json, or stream-json. Stream-json is relevant to wrappers because non-interactive approvals cannot be answered in-band.
    example: gemini -p "list files" --output-format stream-json
    example_description: Runs headless and emits streaming JSON events.
  - param: raw-output
    style: switch
    description: Disables sanitization of model output, including ANSI escape filtering. This is an output security-control switch rather than a tool permission.
    example: gemini --raw-output --accept-raw-output-risk -p "print colored text"
    example_description: Allows raw model output after acknowledging the risk.
  - param: accept-raw-output-risk
    style: switch
    description: Suppresses the warning shown when --raw-output is used.
    example: gemini --raw-output --accept-raw-output-risk -p "emit escape sequences"
    example_description: Acknowledges raw-output risk for automation.
  - param: mcp add --trust
    style: switch
    description: Trusts an MCP server at configuration time, bypassing tool-call confirmation prompts for that server.
    example: gemini mcp add local-docs node server.js --trust
    example_description: Adds a trusted stdio MCP server.
  - param: mcp add --include-tools
    style: switch
    description: Adds an MCP server with a tool allowlist. Accepts a comma-separated list of MCP tool names.
    example: gemini mcp add jira node jira.js --include-tools search,get_issue
    example_description: Exposes only selected tools from the jira server.
  - param: mcp add --exclude-tools
    style: switch
    description: Adds an MCP server with a tool blocklist. Excluded tools win over included tools.
    example: gemini mcp add jira node jira.js --exclude-tools delete_issue
    example_description: Hides a destructive MCP tool from the model.

env_vars:
  - name: GEMINI_SANDBOX
    effect: "Enables sandboxing and optionally selects the backend: true, docker, podman, sandbox-exec, runsc, or lxc. Environment selection has precedence over settings files."
    effect_category: sandbox_control
  - name: GEMINI_SANDBOX_IMAGE
    effect: Selects a custom Docker/Podman image or LXC container name for sandboxing.
    effect_category: sandbox_control
  - name: GEMINI_SANDBOX_PROXY_COMMAND
    effect: Configures the sandbox proxy command used by proxied sandbox profiles.
    effect_category: sandbox_control
  - name: GEMINI_CLI_TRUST_WORKSPACE
    effect: When true, trusts the current workspace for the session, equivalent to --skip-trust.
    effect_category: workspace_trust
  - name: GEMINI_CLI_TRUSTED_FOLDERS_PATH
    effect: Overrides the path of trustedFolders.json.
    effect_category: config_path_override
  - name: GEMINI_CLI_SYSTEM_SETTINGS_PATH
    effect: Overrides the path to the system-wide settings override file.
    effect_category: config_path_override
  - name: GEMINI_CLI_SYSTEM_DEFAULTS_PATH
    effect: Overrides the path to the system-wide defaults file.
    effect_category: config_path_override
  - name: GEMINI_CLI_HOME
    effect: Relocates Gemini CLI user state and config from ~/.gemini to a separate directory.
    effect_category: state_home_relocation
  - name: SEATBELT_PROFILE
    effect: Selects a macOS Seatbelt profile such as permissive-open, permissive-proxied, restrictive-open, restrictive-proxied, strict-open, or strict-proxied.
    effect_category: sandbox_control
  - name: SANDBOX_MOUNTS
    effect: Adds container sandbox mounts as comma-separated from:to:opts entries. Missing opts default to read-only.
    effect_category: sandbox_control
  - name: SANDBOX_FLAGS
    effect: Passes extra flags to Docker or Podman sandbox commands.
    effect_category: sandbox_control
  - name: SANDBOX_SET_UID_GID
    effect: Forces or disables Linux UID/GID mapping in container sandboxes.
    effect_category: sandbox_control
  - name: SANDBOX_PORTS
    effect: Exposes additional ports to the sandbox container.
    effect_category: sandbox_control
  - name: SANDBOX_ENV
    effect: Passes comma-separated key=value environment variables into the sandbox.
    effect_category: sandbox_control
  - name: BUILD_SANDBOX
    effect: Requests automatic local sandbox image build when running from source; npm installs require a prebuilt image.
    effect_category: sandbox_control
  - name: DEBUG
    effect: Enables debug logging. Project .env DEBUG is automatically excluded; use shell env or .gemini/.env for Gemini-specific debug.
    effect_category: none
  - name: GEMINI_CLI
    effect: Set to 1 in subprocesses launched by run_shell_command so child scripts can detect Gemini CLI execution.
    effect_category: other

config_files:
  - os: macos
    user: .gemini/settings.json
    repo: .gemini/settings.json
    notes: User policies are in .gemini/policies/*.toml under the home directory. Admin settings are /Library/Application Support/GeminiCli/settings.json and system-defaults.json; Admin policies are /Library/Application Support/GeminiCli/policies.
  - os: linux
    user: .gemini/settings.json
    repo: .gemini/settings.json
    notes: User policies are in .gemini/policies/*.toml under the home directory. Admin settings are /etc/gemini-cli/settings.json and /etc/gemini-cli/system-defaults.json; Admin policies are /etc/gemini-cli/policies.
  - os: windows
    user: .gemini/settings.json
    repo: .gemini/settings.json
    notes: User path is relative to %USERPROFILE%. Admin settings are C:\ProgramData\gemini-cli\settings.json and system-defaults.json; Admin policies are C:\ProgramData\gemini-cli\policies.

precedence:
  - source: cli
    scope: [approval_mode, sandbox, config_loading, mcp, tool_visibility, trust, other]
    merge_strategy: none
    notes: CLI arguments are session-scoped and override environment variables and settings for the same scalar surface. --policy and --admin-policy add TOML policy sources rather than replacing all policy.
  - source: environment variables
    scope: [sandbox, trust, config_loading, other]
    merge_strategy: none
    notes: Environment variables override settings files for their surfaces and can relocate user/system config paths.
  - source: system_settings
    scope: [general_config, mcp, tool_visibility, sandbox]
    merge_strategy: shallow
    notes: System overrides have final precedence among settings files. Scalars replace lower scopes; arrays and objects are merged, with mcpServers of the same name taking the higher-precedence definition.
  - source: project_config
    scope: [general_config, mcp, tool_visibility, sandbox, hooks, slash_commands, skills]
    merge_strategy: shallow
    notes: Project .gemini/settings.json overrides user settings but is ignored in untrusted folders.
  - source: user_config
    scope: [general_config, mcp, tool_visibility, sandbox]
    merge_strategy: shallow
    notes: User settings override system defaults. The observed local ~/.gemini/settings.json had no permission policy rules; it configured auth selection, preview/session settings, shell color, ripgrep, and UI status.
  - source: system_defaults
    scope: [general_config]
    merge_strategy: shallow
    notes: Lowest settings-file layer after application defaults.
  - source: admin_policy
    scope: [rules]
    merge_strategy: none
    notes: Admin TOML rules have highest policy tier. Standard admin policy directories are subject to ownership/permission checks; supplemental --admin-policy is ignored when standard admin policy files exist.
  - source: user_policy
    scope: [rules]
    merge_strategy: none
    notes: User TOML rules in ~/.gemini/policies/*.toml outrank Workspace, Extension, and Default policy rules.
  - source: workspace_policy
    scope: [rules]
    merge_strategy: none
    notes: Documented tier for .gemini/policies/*.toml, but current docs warn that workspace policies are non-functional.
  - source: extension_policy
    scope: [rules, mcp, tool_visibility]
    merge_strategy: shallow
    notes: Extensions may contribute policy files, tools, MCP servers, commands, hooks, and skills. Local MCP overrides merge restrictively for tool include/exclude lists.
  - source: default_policy
    scope: [rules]
    merge_strategy: none
    notes: Built-in TOML policy files define read-only defaults, write prompts, Plan Mode, YOLO, non-interactive denial, discovered-tool defaults, subagent invocation, sandbox-default, and optional Conseca checker rules.

default_posture: "With no CLI flags, env vars, user config, repo config, managed settings, policies, or trust state changes, Gemini CLI uses default approval mode: read-only/search/context tools are allowed, mutating tools and shell execution ask in interactive mode and are denied in headless mode. Sandboxing is off by default, while folder trust documentation conflicts: the trusted-folders guide says disabled by default, but the generated settings reference lists security.folderTrust.enabled defaulting to true."

cli_zero_permissions:
  supported: true
  invocation: gemini --approval-mode plan --exclude-tools run_shell_command,write_file,replace,web_fetch,activate_skill
  mechanism: CLI-only restrictive baseline using Plan Mode plus temporary block rules for the primary mutating and network-fetch tools. For absolute no-tools behavior, Claudine would need to add a temporary deny-all policy file with --policy.
  limitations: There is no documented top-level --no-tools or empty built-in-tool allowlist flag. Plan Mode still exposes read/search/planning tools and allows plan-file writes under .gemini/tmp. Additional allow rules can be supplied only by deprecated --allowed-tools or by passing policy files; policy files cannot be authored inline on the command line.

agent_permissions:
  allowed: true
  fm_properties:
    - subagent

yolo:
  has_interactive_yolo: true
  has_non_interactive_yolo: true
  mechanism: "--approval-mode=yolo or deprecated --yolo/-y; blocked by security.disableYoloMode, admin.secureModeEnabled, or untrusted-folder restrictions"

policy_engine:
  ergonomic: false
  provides_coverage: false
  gaps:
    - Gemini's TOML policy tier math and dynamic settings-derived priority bands are more complex than Claudine's flat PolicyEngine model.
    - Regex argsPattern, commandRegex, commandPrefix shorthand, toolAnnotations, modes, interactive matching, allowRedirection, and safety_checker hooks are not first-class PolicyEngine concepts.
    - Gemini separates tool visibility from approval through tools.core, tools.exclude, policy deny-without-argsPattern, extension selection, and MCP include/exclude filters.
    - Folder trust safe mode can disable project settings, custom commands, MCP servers, hooks, memory loading, and auto-acceptance outside normal rule evaluation.
    - Sandbox state, sandbox expansion, tool-level sandboxing, OS/container backend choice, and network/mount controls are not represented as static permission rules.
    - MCP server trust, OAuth, server allow/exclude filters, per-server tool filters, and extension override merge semantics require provider-specific modeling.
    - Subagent invocation is governed both as invoke_agent and as virtual tool names, and rules can also be scoped by the calling subagent.
    - Non-interactive mode changes ask_user into deny at runtime, which cannot be represented as a source-level static decision without context.
    - Admin/system settings and secure mode can constrain user and repo settings in ways that are not ordinary policy overrides.

permission_entities:
  - entity: tool
    native_names: ["toolName", "tools.core", "tools.exclude", "--allowed-tools", "--exclude-tools"]
    notes: Built-in tools and discovered tools can be allowed, asked, denied, hidden, or filtered. Deprecated tools.allowed/tools.exclude still exist as compatibility surfaces.
  - entity: tool_group
    native_names: ["read-only tools", "write tools", "discovered_tool_*", "toolAnnotations"]
    notes: Groups are mostly implicit through tool names, wildcards, annotations such as readOnlyHint, and built-in policy files.
  - entity: command
    native_names: ["run_shell_command", "commandPrefix", "commandRegex", "allowRedirection", "tools.core", "tools.exclude"]
    notes: Shell policy can match prefix or regex, and redirection requires explicit rule permission unless YOLO allows it.
  - entity: path
    native_names: ["argsPattern", "file_path", "dir_path", "tools.sandboxAllowedPaths", "SANDBOX_MOUNTS", "includeDirectories"]
    notes: Path policy is mostly regex over tool arguments plus sandbox/mount/workspace controls, not a separate path-rule language.
  - entity: workspace
    native_names: ["includeDirectories", "--include-directories", "security.folderTrust.enabled", "trustedFolders.json"]
    notes: Workspace scope affects file access, memory discovery, project settings, and trust gating.
  - entity: mcp_server
    native_names: ["mcpServers", "mcp.allowed", "mcp.excluded", "--allowed-mcp-server-names", "trust"]
    notes: Servers can be filtered, trusted, and merged by name across settings scopes.
  - entity: mcp_tool
    native_names: ["mcpName", "mcp_*", "mcp_server_*", "includeTools", "excludeTools"]
    notes: MCP tools are registered as fully qualified names mcp_{serverName}_{toolName}; mcpName is the recommended policy field.
  - entity: mcp_resource
    native_names: ["list_mcp_resources", "read_mcp_resource", "@server://resource/path"]
    notes: Resources are discovered and can be listed/read through built-in tools when the server connects.
  - entity: agent
    native_names: ["invoke_agent", "remote agents"]
    notes: Agent delegation is a tool call and remote agents can require user confirmation.
  - entity: subagent
    native_names: ["subagent", "agent_name", "complete_task"]
    notes: Policy rules can target subagents as virtual tool names or scope tool calls by the executing subagent.
  - entity: mode
    native_names: ["general.defaultApprovalMode", "--approval-mode", "default", "auto_edit", "plan", "yolo"]
    notes: Modes select built-in policy behavior and can be used as rule conditions.
  - entity: approval_category
    native_names: ["allow", "ask_user", "deny"]
    notes: ask_user becomes deny in non-interactive mode.
  - entity: sandbox
    native_names: ["tools.sandbox", "security.toolSandboxing", "GEMINI_SANDBOX", "sandbox-default.toml", "sandbox_expansion_required"]
    notes: Sandboxing is separate from approval policy and may be full-process or tool-level.
  - entity: hook
    native_names: ["hooks", "hooksConfig.enabled", "gemini hooks migrate"]
    notes: Hooks can intercept/observe CLI behavior and are disabled in untrusted-folder safe mode.
  - entity: extension
    native_names: ["extensions", "--extensions", "security.blockGitExtensions", "security.allowedExtensions"]
    notes: Extensions can contribute tools, MCP servers, hooks, skills, commands, and policies; extension source controls are security relevant.
  - entity: slash_command
    native_names: ["/permissions", "/mcp", "/tools", "/settings", "/agents reload", "/extensions reload"]
    notes: Interactive commands can manage trust, inspect tools/MCP, reload resources, and change settings.
  - entity: unknown
    native_names: ["security.enableConseca", "safety_checker"]
    notes: Context-aware security and in-process safety checkers add dynamic decisions not expressible as plain static rules.

approval_modes:
  - name: default
    effect: Read-only/search/context tools are allowed; file edits, shell, web_fetch, activate_skill, and many discovered or remote actions ask in interactive mode and deny in headless mode.
    interactive: true
    non_interactive: true
    aliases: ["default", "general.defaultApprovalMode=default"]
  - name: auto_edit
    effect: Auto-approves selected edit tools such as write_file and replace when safety checks allow the path; shell still asks unless separately allowed.
    interactive: true
    non_interactive: true
    aliases: ["auto_edit", "autoEdit", "general.defaultApprovalMode=auto_edit"]
  - name: plan
    effect: Research/read-only mode. Denies most tools, asks for selected read-only MCP and web/skill tools in interactive mode, allows only specific plan-file writes under .gemini/tmp, and permits plan transitions according to interactive/headless policy.
    interactive: true
    non_interactive: true
    aliases: ["plan", "--approval-mode plan", "/plan"]
  - name: yolo
    effect: Allows all tools with redirection except ask_user and interactive plan-mode transitions, subject to trust, admin, sandbox, and explicit deny rules.
    interactive: true
    non_interactive: true
    aliases: ["yolo", "--approval-mode yolo", "--yolo", "-y"]

rule_model:
  decisions: ["allow", "ask_user", "deny"]
  syntax: "TOML [[rule]] blocks with toolName or toolName array, optional subagent, mcpName, toolAnnotations, argsPattern, commandPrefix, commandRegex, decision, priority 0-999, denyMessage, modes, interactive, allowRedirection, and optional safety_checker."
  precedence: "Highest final priority wins. Current bundled policy comments and source use tier bases Default 1, Extension 2, Workspace 3, User 4, Admin 5, with final_priority = tier_base + priority/1000. Settings-derived dynamic rules occupy User-tier sub-bands such as 4.95 persistent always-allow, 4.9 MCP excluded, 4.4 CLI exclude-tools, 4.3 CLI allowed-tools, 4.2 MCP trust=true, and 4.1 MCP allowed."
  merge_semantics: "Policy TOML rules are additive and conflict by priority, not by deep merge. Settings files merge by scope: scalars replace, arrays such as includeDirectories concatenate, objects such as mcpServers merge by key with higher-precedence definitions replacing lower ones. Extension MCP includeTools intersect, excludeTools union, and exclude wins."
  matcher_semantics: "toolName supports * and MCP FQN wildcards; mcpName targets MCP servers and can be *. argsPattern and commandRegex are regular expressions over stable JSON argument strings. commandPrefix is prefix matching for shell commands. toolAnnotations requires all listed annotation key-values. modes accepts policy mode names, with docs using autoEdit while CLI uses auto_edit."
  default_decision: "When no user/admin rule matches, built-in default policy files decide: read-only tools allow, write/shell/web_fetch/activate_skill ask in interactive and deny in headless, Plan Mode denies most actions, and YOLO allows nearly all actions."

tool_visibility:
  supported: true
  mechanisms:
    - "tools.core is an allowlist for all built-in tools and can narrow run_shell_command by command prefix."
    - "tools.exclude and --exclude-tools are deprecated blocklist surfaces; blocklist entries take precedence over allowlist entries."
    - "A policy deny rule without argsPattern excludes the denied tool from the model's memory."
    - "MCP includeTools and excludeTools filter tools before exposure; excludeTools wins."
    - "--extensions can limit extension-contributed tools, MCP servers, commands, hooks, skills, and policies for one session."
  notes: "Tool visibility is distinct from approval. A visible tool may still ask, while a hidden tool is removed from the model's option set."

sandbox:
  supported: true
  modes: ["full-process tools.sandbox", "tool-level security.toolSandboxing", "sandbox-default modes: plan, default, accepting_edits", "sandbox expansion"]
  backends: ["macOS Seatbelt", "Docker", "Podman", "Windows Native Sandbox", "gVisor/runsc on Linux", "LXC/LXD on Linux"]
  filesystem_control: "Full-process/container sandboxing mounts the workspace at the same absolute path and limits access to the workspace plus configured mounts/allowed paths. macOS Seatbelt profiles range from write-restricted to strict read/write restriction. Windows Native Sandbox uses persistent Low Mandatory Level integrity changes."
  network_control: "Sandbox defaults disable network in sandbox-default.toml. Seatbelt profiles choose open or proxied network. Container sandboxes can use proxy/network settings, SANDBOX_PORTS, SANDBOX_ENV, and custom flags."
  notes: "Sandboxing is separate from approval mode. Dynamic sandbox expansion can ask for one-run path or network expansion when a command fails or is predicted to need extra access. Sandboxing reduces risk but does not eliminate all risk."

trust_and_admin:
  folder_trust: "Folder trust stores decisions in ~/.gemini/trustedFolders.json by default. The trusted-folders guide says the feature is disabled by default, while the generated settings reference says security.folderTrust.enabled defaults to true; local observed trustedFolders.json exists and records TRUST_PARENT/TRUST_FOLDER entries. Untrusted safe mode ignores workspace settings and .env, blocks extension management, disables tool auto-acceptance and automatic memory, prevents MCP connections, and skips custom commands."
  managed_policy: "System defaults and system overrides are JSON settings layers. Admin TOML policies live in OS-specific system directories or supplemental --admin-policy/adminPolicyPaths. Standard admin policy directories require secure ownership/permissions and supplemental admin policies are ignored if standard policy files exist."
  safe_mode: "Headless untrusted workspaces exit with FatalUntrustedWorkspaceError when folder trust is enabled; --skip-trust or GEMINI_CLI_TRUST_WORKSPACE=true trusts only for the session. admin.secureModeEnabled also disables YOLO in source."
  notes: "Enterprise docs explicitly classify system settings as policy-enforcement aids, not a foolproof local security boundary against users with local administrative privileges."

mcp_permissions:
  supported: true
  server_filters:
    - "mcp.allowed only connects listed configured servers."
    - "mcp.excluded disables listed servers."
    - "--allowed-mcp-server-names restricts servers for the session."
    - "mcpServers definitions merge by server name, with higher-precedence settings replacing lower definitions."
  tool_filters:
    - "includeTools exposes only selected tools from a server."
    - "excludeTools hides selected tools and wins over includeTools."
    - "Policy rules can use mcpName alone, mcpName plus toolName, mcpName='*', or MCP FQN wildcards."
    - "mcp add --include-tools and --exclude-tools write server-specific filters."
  trust_model: "mcpServers.<name>.trust=true bypasses confirmation for that server's tools. Interactive prompts can also allow once, always allow a tool, or always allow a server. Untrusted folders do not connect MCP servers."
  notes: "MCP servers are external processes or remote endpoints. Stdio servers inherit a sanitized environment unless env is explicitly configured. OAuth remote servers store tokens under the Gemini user directory; current docs say mcp-oauth-tokens.json, while observed local state uses mcp-oauth-tokens-v2.json. MCP tools must be available inside the sandbox when sandboxing is enabled; otherwise they can fail."

headless_behavior: "In -p/--prompt non-interactive mode, ask_user is treated as deny and approval/trust dialogs cannot be answered. If folder trust is enabled and the folder is untrusted, Gemini CLI exits with FatalUntrustedWorkspaceError unless --skip-trust or GEMINI_CLI_TRUST_WORKSPACE=true is used; OAuth MCP flows requiring a browser are not suitable for headless environments."

approval_persistence: "Interactive confirmation supports one-time approval, always allow a tool, and always allow a server. Permanent tool approvals are available only when security.enablePermanentToolApproval is true; autoAddToPolicyByDefault can make permanent approval the default for low-risk tools in trusted workspaces. Trust decisions persist in trustedFolders.json."

protected_paths:
  - "gha-creds-*.json"

security_posture: "Gemini CLI combines advisory approval prompts, a static client-side policy engine, folder trust gating, managed settings/policy layers, and optional OS/container-enforced sandboxing. Without sandboxing it is not an OS security boundary; enterprise docs warn that local administrators can bypass managed settings."

changes:
  - "Updated research against current npm @google/gemini-cli 0.49.0 while noting the locally installed gemini binary is 0.46.0."
  - "Added schema-valid OS-specific config_files records and removed the old invalid os: all entry."
  - "Added current source-observed top-level --exclude-tools, --acp, raw-output, and MCP add trust/include/exclude flags to cli_params."
  - "Recorded local ~/.gemini as symlinks into ~/.claudine/.gemini and observed current settings/trustedFolders shape without reading credential or token contents."
  - "Corrected zero-permissions guidance to use Plan Mode plus explicit temporary tool exclusions, and documented that true no-tools requires a temporary --policy file because there is no no-tools CLI flag."
  - "Retained and clarified the folder trust documentation conflict: trusted-folders says disabled by default, settings reference says default true."
  - "Refreshed built-in tool inventory to include current task tracker, topic, internal docs, planning, MCP resource, and subagent lifecycle tools."
  - "Added current sandbox expansion, SANDBOX_PORTS, SANDBOX_ENV, GEMINI_SANDBOX_PROXY_COMMAND, Windows Native Sandbox, and tool-level sandboxing details."
  - "Added current MCP OAuth/token, environment sanitization, per-server trust, includeTools/excludeTools, and restrictive extension merge semantics."
  - "Confirmed workspace policy tier remains documented as non-functional and should not be used as current truth."

requires_claudine_update: true
reason: "Claudine's PolicyEngine cannot accurately model Gemini CLI 0.49.0 without provider-specific extensions for rule tier math, dynamic settings-derived priorities, commandPrefix/commandRegex/argsPattern matching, safety checkers, tool visibility, folder trust safe mode, sandbox expansion/backends, MCP trust and filters, subagent scoping, and headless ask_user-to-deny behavior."
---

# Gemini CLI Permissions and Security Controls

## Introduction to Gemini CLI Permissions

Gemini CLI permissions are layered. The broad session posture is the approval mode (`default`, `auto_edit`, `plan`, or `yolo`). Fine-grained decisions come from the Gemini CLI Policy Engine, which loads TOML rules and returns `allow`, `ask_user`, or `deny` for matching tool calls. Separate security-control layers govern sandboxing, folder trust, extensions, MCP server/tool visibility, raw output, and system/admin settings.

Configuration files can define permissions in two main formats:

- JSON settings files set approval defaults, sandbox settings, tool visibility, MCP filters, folder trust, extension controls, and admin controls.
- TOML policy files define rule-level tool decisions with matchers for tools, commands, MCP servers, subagents, mode, interactivity, annotations, and arguments.

Environment variables influence the same posture. `GEMINI_SANDBOX` enables/selects sandboxing, `GEMINI_CLI_TRUST_WORKSPACE=true` bypasses folder trust for one session, `GEMINI_CLI_HOME` relocates user state, and `GEMINI_CLI_SYSTEM_SETTINGS_PATH` or `GEMINI_CLI_SYSTEM_DEFAULTS_PATH` relocates system settings. Sandbox-specific variables such as `SEATBELT_PROFILE`, `SANDBOX_MOUNTS`, `SANDBOX_FLAGS`, `SANDBOX_PORTS`, and `SANDBOX_ENV` tune the execution sandbox.

The current package's CLI reference lists `--approval-mode`, `--yolo`, `--sandbox`, `--skip-trust`, `--allowed-mcp-server-names`, `--allowed-tools`, `--extensions`, `--include-directories`, `--prompt`, and output controls. Current bundled source also exposes `--policy`, `--admin-policy`, `--exclude-tools`, `--acp`, `--raw-output`, and `--accept-raw-output-risk`. CLI flags are session-scoped and have the highest precedence for their surfaces, followed by environment variables, then system override settings, project settings, user settings, system defaults, and hardcoded defaults. Policy TOML has its own tier system: Admin > User > Workspace > Extension > Default, though Workspace policy is documented as non-functional in current docs.

Permission/approval policy is not the same as tool visibility. Approval policy decides whether a visible tool call runs automatically, prompts, or is denied. Visibility controls decide what the model sees at all: `tools.core`, `tools.exclude`, bare policy `deny` rules, MCP `includeTools`/`excludeTools`, `mcp.allowed`, `mcp.excluded`, and `--extensions` can remove tools or entire extension surfaces before the model can request them.

## Permissions Use Cases

### Default

If no environment variables, config files, trust state, or CLI switches provide permission guidance, Gemini CLI starts in `default` approval mode. Built-in read/search/context tools such as `read_file`, `read_many_files`, `glob`, `grep_search`, `list_directory`, `google_web_search`, `list_mcp_resources`, and `read_mcp_resource` are allowed by default. Mutating or externally risky tools such as `write_file`, `replace`, `run_shell_command`, `web_fetch`, and `activate_skill` ask in interactive mode and are denied in non-interactive mode.

Sandboxing is off by default. Folder trust is ambiguous in documentation: the trusted-folders page says disabled by default, while the generated settings reference says `security.folderTrust.enabled` defaults to `true`. The local machine has a populated `trustedFolders.json`, so trust is in active use locally, but the user settings file does not explicitly enable it.

In Claudine's `PolicyEngine`, the default can be approximated as:

- Read/search/context tools: `allow`
- File edits and shell commands: `ask` in interactive mode, `deny` in non-interactive mode
- MCP server tools: `ask` unless trusted or covered by allow policy
- Subagent invocation: `allow` for local subagents in default/auto-edit/yolo, with subagent tool calls checked separately

That is not ergonomic. Claudine can express a simple allow/ask/deny posture, but Gemini CLI's real default also depends on approval mode, interactivity, trust state, built-in policies, dynamic settings-derived rules, and sandbox state. Without changes, PolicyEngine can describe the broad use case but cannot safely round-trip or mutate every provider-native permission surface.

### Whitelisting

For a CLI-only, session-scoped locked-down launch, the best available baseline is:

```bash
gemini --approval-mode plan --exclude-tools run_shell_command,write_file,replace,web_fetch,activate_skill
```

This starts in Plan Mode and adds temporary exclusions for the main mutating and network-fetch tools. It does not hide every built-in tool. Read/search/context/planning tools remain available, and Plan Mode can still write approved `.md` plan files under `.gemini/tmp`.

For a true deny-by-default baseline, use a temporary policy file and pass it with `--policy`:

```toml
[[rule]]
toolName = "*"
decision = "deny"
priority = 900

[[rule]]
toolName = ["read_file", "glob", "grep_search", "list_directory"]
decision = "allow"
priority = 950
```

Then launch:

```bash
gemini --approval-mode plan --policy /tmp/claudine-gemini-policy.toml
```

Additional permissions can be granted with CLI flags or policy files:

```bash
gemini --approval-mode plan --allowed-mcp-server-names docs
gemini --policy ./allow-git-status.toml
gemini --allowed-tools read_file,glob
gemini --extensions corp-readonly
```

PolicyEngine could model this as a deny-all base plus explicit allow/ask rules. It is still not ergonomic because Gemini CLI has no inline rule syntax and no documented `--no-tools` flag. Claudine would need to create temporary policy files and possibly isolate `GEMINI_CLI_HOME` to avoid mutating user config. Without changes, PolicyEngine cannot fully define a no-tools baseline for Gemini CLI using only current provider-native CLI switches.

### YOLO

YOLO mode is available in interactive and non-interactive sessions through `--approval-mode=yolo`, `--approval-mode yolo`, `--yolo`, or `-y`. `--yolo` and `-y` are deprecated aliases. Source rejects simultaneous `--yolo` and `--approval-mode`.

YOLO allows nearly all tools and sets `allowRedirection = true` for shell commands. The built-in `yolo.toml` still keeps `ask_user` as `ask_user` in interactive mode and denies interactive plan-mode transitions to avoid state conflicts. YOLO can be blocked by `security.disableYoloMode`, `admin.secureModeEnabled`, untrusted-folder restrictions, explicit higher-tier deny policy, and sandbox boundaries.

### Root User

Current public docs and bundled source do not document special root-user behavior for Gemini CLI approvals. There is no documented UID-based refusal of YOLO mode. A root-run session can request YOLO unless an admin/security setting or policy disables it. Running as root raises the stakes of any non-sandboxed shell/file action; it does not make Gemini's advisory approval prompts an OS boundary.

### Configuring the Default

User-scope defaults live in `~/.gemini/settings.json` and `~/.gemini/policies/*.toml`. Repo-scope settings live in `.gemini/settings.json`; repo-scope policy files are documented as `.gemini/policies/*.toml` but currently non-functional. Admin settings and policies live in OS-specific system paths.

Example user settings:

```json
{
  "general": {
    "defaultApprovalMode": "plan"
  },
  "security": {
    "disableYoloMode": true,
    "enablePermanentToolApproval": false
  }
}
```

Example tool visibility and MCP settings:

```json
{
  "tools": {
    "core": ["read_file", "glob", "grep_search", "run_shell_command(git status)"]
  },
  "mcp": {
    "allowed": ["corp-docs"]
  },
  "mcpServers": {
    "corp-docs": {
      "command": "/opt/corp-docs/start.sh",
      "includeTools": ["search", "read_doc"],
      "trust": false
    }
  }
}
```

Example TOML policy:

```toml
[[rule]]
toolName = "run_shell_command"
commandPrefix = "git status"
decision = "allow"
priority = 200
modes = ["default", "autoEdit"]

[[rule]]
toolName = ["write_file", "replace"]
argsPattern = '"file_path":".*\\.env"'
decision = "deny"
priority = 900
denyMessage = "Writing .env files is not allowed."
```

### Extending the Base

User defaults can be narrowed by project settings or overridden by CLI flags. For example, a user can set `general.defaultApprovalMode = "auto_edit"` and a single Claudine wrapper run can use `gemini --approval-mode plan` without mutating the user's file.

MCP settings merge by server name. A system settings file can define `mcp.allowed = ["corp-tools"]` and the canonical `mcpServers.corp-tools` command. A user cannot override that server definition at lower precedence, and a different user-defined server will be blocked if it is not in `mcp.allowed`.

Tool visibility can also narrow a broader policy. A user policy might allow `run_shell_command` for `git`, but a project settings file with `tools.core = ["read_file", "glob"]` removes shell from the model's visible tool set in that project. In untrusted folders, project settings are ignored entirely, so this narrowing does not apply until the folder is trusted.

## Tools and Permissions

Gemini CLI's default built-in tools include:

| Category | Tools |
| --- | --- |
| Execution | `run_shell_command` |
| File system | `glob`, `grep_search`, `list_directory`, `read_file`, `read_many_files`, `replace`, `write_file` |
| Interaction | `ask_user`, `write_todos` |
| Task tracker | `tracker_create_task`, `tracker_update_task`, `tracker_get_task`, `tracker_list_tasks`, `tracker_add_dependency`, `tracker_visualize` |
| MCP resources | `list_mcp_resources`, `read_mcp_resource` |
| Memory/internal docs | `activate_skill`, `get_internal_docs` |
| Planning | `enter_plan_mode`, `exit_plan_mode` |
| System/subagent | `complete_task`, `invoke_agent` |
| Topic/status | `update_topic` |
| Web | `google_web_search`, `web_fetch` |

Permissions map to tool calls through the Policy Engine. Each tool call is matched against active rules. `toolName` can target exact built-in names, arrays of names, wildcards, MCP fully qualified names, discovered tool patterns, or subagent virtual names. Shell commands can be matched with `commandPrefix` or `commandRegex`. Tool arguments can be matched by regex through `argsPattern` against a stable JSON representation.

Native permission entities include tools, implicit tool groups, shell commands, filesystem paths through arguments and sandbox mounts, workspaces, MCP servers, MCP tools, MCP resources, subagents, modes, approval decisions, sandboxes, hooks, extensions, and slash commands. Gemini CLI also has dynamic safety checker entities such as `allowed-path` and `conseca`.

Rule decisions are `allow`, `ask_user`, and `deny`. `deny` rules without `argsPattern` hide the tool from model memory. `ask_user` prompts in interactive mode and becomes `deny` in headless mode. Priority resolves conflicts: the highest final priority wins. Tier base values in bundled policy comments and current source are Default 1, Extension 2, Workspace 3, User 4, Admin 5. The policy-engine docs contain stale example arithmetic for some tiers; the bundled policy files and source are internally consistent on Admin 5/User 4/Workspace 3/Extension 2/Default 1.

Approvals can persist. One-time choices are session-local. "Always allow this tool/server" can create persistent dynamic allow rules when enabled. Mode-aware persistence expands to the current mode and more permissive modes, so a default-mode persistent approval applies to default, auto-edit, and YOLO, while a Plan Mode persistent approval applies to all modes.

## Sandboxing, Trust, and Administrative Controls

Sandbox mode is separate from approval mode. `--sandbox`, `GEMINI_SANDBOX`, or `tools.sandbox` enable full-process sandboxing. `security.toolSandboxing` enables tool-level sandboxing. Backends include macOS Seatbelt, Docker, Podman, Windows Native Sandbox, gVisor/runsc on Linux, and LXC/LXD on Linux. macOS profiles range from permissive write restrictions to strict read/write restrictions. Container sandboxes mount the workspace at the same absolute path. Windows Native Sandbox uses Low Mandatory Level integrity labels, which can persist after the session.

Network control depends on backend and profile. Built-in `sandbox-default.toml` disables network for plan/default/accepting-edits modes. Seatbelt profiles have open and proxied variants. Container mode supports custom network, proxy, ports, env, flags, and mounts.

Sandbox expansion is a dynamic permission mechanism. If a sandboxed command fails because of restricted paths/network, or is predicted to need extra permissions, Gemini CLI can show a Sandbox Expansion Request. Approval applies to that specific run.

Folder trust gates project-local surfaces. Untrusted safe mode ignores `.gemini/settings.json` and project `.env`, blocks extension install/update/uninstall, disables tool auto-acceptance and automatic memory loading, prevents MCP servers from connecting, and skips custom commands. The `/permissions` slash command manages trust interactively; headless runs must use `--skip-trust` or `GEMINI_CLI_TRUST_WORKSPACE=true` when folder trust would otherwise prompt.

Managed/admin controls include system defaults, system overrides, Admin policy TOML, `security.disableYoloMode`, extension source restrictions, authentication enforcement, and `admin.secureModeEnabled` in source. Enterprise docs explicitly warn these are not foolproof against users with local administrative privileges.

The only provider-reserved protected file pattern observed in current bundled policy is `gha-creds-*.json`, denied by `sandbox-default.toml`.

Gemini CLI's honest security posture is a combination: advisory UX prompts, static client-side policy, trust gating, admin-managed settings, and optional OS/container-enforced sandboxing. Without sandboxing, the approval system is not an OS-enforced boundary.

## MCP and Permissions

MCP permissions combine server filtering, tool filtering, trust, and policy rules.

Server-level controls:

- `mcp.allowed` allows only named configured servers.
- `mcp.excluded` disables named servers.
- `--allowed-mcp-server-names` restricts servers for one session.
- `mcpServers.<name>.trust=true` bypasses confirmation for that server's tools.

Tool-level controls:

- `includeTools` exposes only selected tools.
- `excludeTools` hides selected tools and wins over `includeTools`.
- Policy rules can target `mcpName`, `mcpName` plus `toolName`, `mcpName = "*"`, or FQN wildcard syntax such as `mcp_*`.

MCP can be made safer by combining an allowlisted server catalog with per-server `includeTools`, untrusted `trust: false`, and policy rules that ask or deny by `mcpName`. Enterprise docs recommend defining canonical servers and `mcp.allowed` together; defining servers without `mcp.allowed` still lets users add unrelated servers.

MCP resources are discovered with the server and accessed through `list_mcp_resources`, `read_mcp_resource`, or `@server://resource/path`. MCP tool schemas are sanitized before Gemini API use. Stdio MCP servers are separate subprocesses with environment sanitization; explicit `env` entries are treated as consent and are not redacted. OAuth remote MCP flows require browser/local redirect support and are unsuitable for headless environments. MCP servers must be available inside the sandbox when sandboxing is enabled, otherwise they can fail.

## Non-Interactive Behavior

Non-interactive mode is triggered by `-p`/`--prompt` or a non-TTY/piped mode. Approval prompts cannot be answered there. Policy `ask_user` becomes `deny`; built-in non-interactive policy denies `ask_user`, `write_file`, `replace`, `run_shell_command`, `activate_skill`, and `web_fetch` when they would otherwise ask.

Gemini CLI does not expose a general programmatic approval channel for headless runs. A Claudine wrapper should avoid any posture that can require interactive approval unless it also supplies explicit allow/deny policy. If folder trust is enabled and the workspace is untrusted, headless execution exits with `FatalUntrustedWorkspaceError` unless trust is bypassed for the session.

## Changelog

- 2026-07-03: Refreshed against npm `@google/gemini-cli` 0.49.0, current bundled docs/source, observed local Gemini config, and schema requirements.
- 2026-07-03: Added CLI surfaces missing from the previous document: `--exclude-tools`, `--acp`, raw-output controls, `--policy`, `--admin-policy`, and MCP add trust/include/exclude flags.
- 2026-07-03: Replaced invalid `os: all` frontmatter with macOS/Linux/Windows-specific config file records.
- 2026-07-03: Clarified that Plan Mode plus temporary exclusions is the strongest CLI-only lockdown, while true deny-all requires a temporary policy file passed with `--policy`.
- 2026-07-03: Documented current sandbox expansion, Windows Native Sandbox, sandbox env vars, current MCP OAuth/trust/filter behavior, and the folder-trust docs conflict.
- 2026-07-02: Previous research captured the merged permissions topic and identified PolicyEngine coverage gaps.

## Sources

- [Gemini CLI npm package `@google/gemini-cli`](https://www.npmjs.com/package/@google/gemini-cli)
- [Gemini CLI repository](https://github.com/google-gemini/gemini-cli)
- [Gemini CLI policy engine docs](https://geminicli.com/docs/reference/policy-engine)
- [Gemini CLI CLI reference](https://geminicli.com/docs/cli/cli-reference)
- [Gemini CLI settings docs](https://geminicli.com/docs/cli/settings)
- [Gemini CLI configuration docs](https://geminicli.com/docs/reference/configuration)
- [Gemini CLI sandbox docs](https://geminicli.com/docs/cli/sandbox)
- [Gemini CLI trusted folders docs](https://geminicli.com/docs/cli/trusted-folders)
- [Gemini CLI MCP server docs](https://geminicli.com/docs/tools/mcp-server)
- [Gemini CLI tools reference](https://geminicli.com/docs/reference/tools)
- [Gemini CLI shell tool docs](https://geminicli.com/docs/tools/shell)
- [Gemini CLI enterprise controls docs](https://geminicli.com/docs/cli/enterprise)
- [Gemini CLI subagents docs](https://geminicli.com/docs/core/subagents)
