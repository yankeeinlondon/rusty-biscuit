---
$schema: ./_schema.yaml
created: 2026-07-01
last_updated: 2026-07-02
agent: open_code
model: kimi-for-coding/k2p7

cli_params:
  - param: approval-mode
    style: switch
    description: Set the session approval mode. Values are default (prompt for approval), auto_edit (auto-approve edit tools), plan (read-only mode), and yolo (auto-approve all tools). Overrides general.defaultApprovalMode for this session.
    example: gemini --approval-mode plan
    example_description: Starts a read-only planning session where write operations require explicit approval.
  - param: yolo
    style: switch
    description: Deprecated alias for --approval-mode=yolo. Auto-approves all tool actions for the session.
    example: gemini -y -p "deploy to staging"
    example_description: Runs a headless prompt with all permission prompts auto-approved.
  - param: sandbox
    style: switch
    description: Enable sandboxed execution. Accepts a boolean or a provider such as docker, podman, sandbox-exec, runsc, or lxc.
    example: gemini -s -p "run the test suite"
    example_description: Runs the session inside the default sandbox provider for the platform.
  - param: skip-trust
    style: switch
    description: Trust the current workspace for this session, bypassing the folder trust dialog.
    example: gemini --skip-trust -p "summarize"
    example_description: Runs non-interactively in a folder without prompting for trust.
  - param: include-directories
    style: switch
    description: Add additional directories to the workspace. Repeatable or comma-separated.
    example: gemini --include-directories ../lib,../docs
    example_description: Expands read/write scope to sibling directories for this session.
  - param: allowed-mcp-server-names
    style: switch
    description: Restrict which configured MCP servers are available. Repeatable or comma-separated.
    example: gemini --allowed-mcp-server-names github,slack
    example_description: Only allows tools from the listed MCP servers for this session.
  - param: allowed-tools
    style: switch
    description: Deprecated tools allowlist. Use the Policy Engine instead.
    example: gemini --allowed-tools read_file
    example_description: Legacy way to allow specific tools without confirmation.
  - param: policy
    style: switch
    description: Additional policy files or directories to load (User tier). Repeatable or comma-separated.
    example: gemini --policy ./gemini-policies
    example_description: Loads supplemental User-tier policy TOML files for this session.
  - param: admin-policy
    style: switch
    description: Additional admin policy files or directories to load (Admin tier). Ignored if standard system policy directories already contain .toml files.
    example: gemini --admin-policy /etc/gemini-cli/policies
    example_description: Loads enterprise policy TOML files at Admin tier.
  - param: prompt
    style: switch
    description: Run in non-interactive (headless) mode with the given prompt. In this mode ask_user decisions become deny and interactive trust/approval dialogs are skipped.
    example: gemini -p --approval-mode plan "explain the auth module"
    example_description: Runs a headless read-only task where mutating tools are denied instead of prompted.

env_vars:
  - name: GEMINI_SANDBOX
    effect: Enables sandboxing and optionally selects the provider (true, docker, podman, sandbox-exec, runsc, or lxc).
  - name: GEMINI_SANDBOX_IMAGE
    effect: Specifies a custom container image for Docker or Podman sandboxing.
  - name: GEMINI_CLI_TRUST_WORKSPACE
    effect: When set to true, trusts the current workspace for the session, equivalent to --skip-trust.
  - name: GEMINI_CLI_TRUSTED_FOLDERS_PATH
    effect: Overrides the default path for the trusted folders registry (~/.gemini/trustedFolders.json).
  - name: GEMINI_CLI_SYSTEM_SETTINGS_PATH
    effect: Overrides the path to the system-wide settings override file.
  - name: GEMINI_CLI_SYSTEM_DEFAULTS_PATH
    effect: Overrides the path to the system-wide defaults file.
  - name: GEMINI_CLI_HOME
    effect: Redirects all user-scoped config and state to a different directory, useful for isolation.
  - name: SEATBELT_PROFILE
    effect: Selects a macOS Seatbelt sandbox profile (permissive-open, permissive-proxied, restrictive-open, restrictive-proxied, strict-open, strict-proxied).
  - name: SANDBOX_MOUNTS
    effect: Comma-separated list of host:container:opts mounts to add to a container sandbox.
  - name: SANDBOX_FLAGS
    effect: Passes custom flags into the docker or podman sandbox command.
  - name: SANDBOX_SET_UID_GID
    effect: Enables or disables host UID/GID mapping for Linux sandboxes.
  - name: BUILD_SANDBOX
    effect: When set, builds a local sandbox image from .gemini/sandbox.Dockerfile automatically.
  - name: DEBUG
    effect: Enables debug logging; does not read DEBUG from project .env files.

config_files:
  - os: all
    user: ~/.gemini/settings.json
    repo: .gemini/settings.json
    notes: User policies live in ~/.gemini/policies/*.toml. Workspace policies in .gemini/policies/*.toml are currently disabled. System override paths vary by OS.
  - os: linux
    user: ~/.gemini/settings.json
    repo: .gemini/settings.json
    notes: System defaults at /etc/gemini-cli/system-defaults.json; system overrides at /etc/gemini-cli/settings.json; Admin policies at /etc/gemini-cli/policies.
  - os: macos
    user: ~/.gemini/settings.json
    repo: .gemini/settings.json
    notes: System defaults at /Library/Application Support/GeminiCli/system-defaults.json; system overrides at /Library/Application Support/GeminiCli/settings.json; Admin policies at /Library/Application Support/GeminiCli/policies.
  - os: windows
    user: ~/.gemini/settings.json
    repo: .gemini/settings.json
    notes: System defaults at C:\ProgramData\gemini-cli\system-defaults.json; system overrides at C:\ProgramData\gemini-cli\settings.json; Admin policies at C:\ProgramData\gemini-cli\policies.

precedence:
  - source: cli
    scope: [approval_mode, sandbox, rules, mcp, tool_visibility, trust]
    merge_strategy: none
    notes: CLI flags are temporary session overrides and beat environment variables and file config. --policy adds User-tier TOML; --admin-policy adds Admin-tier TOML unless standard system policy dirs already contain .toml files.
  - source: environment variables
    scope: [sandbox, trust, config_paths]
    merge_strategy: none
    notes: GEMINI_SANDBOX, GEMINI_CLI_TRUST_WORKSPACE, and path overrides take precedence over settings files.
  - source: system settings override file
    scope: [settings, mcp_servers, tools, security]
    merge_strategy: none
    notes: System-wide settings.json overrides user and project settings for scalar values.
  - source: project settings file
    scope: [settings, mcp_servers, tools, security]
    merge_strategy: shallow
    notes: .gemini/settings.json overrides user settings. Arrays/objects like includeDirectories and mcpServers merge.
  - source: user settings file
    scope: [settings, mcp_servers, tools, security]
    merge_strategy: shallow
    notes: ~/.gemini/settings.json overrides system defaults. mcpServers merge by server name.
  - source: system defaults file
    scope: [settings]
    merge_strategy: none
    notes: Baseline machine-wide defaults; lowest precedence among settings files.
  - source: admin_policy
    scope: [rules]
    merge_strategy: none
    notes: Admin-tier TOML rules (standard system dirs or --admin-policy) have the highest rule priority, but are ignored if standard system dirs already contain .toml policies.
  - source: user_policy
    scope: [rules]
    merge_strategy: none
    notes: User-tier TOML rules in ~/.gemini/policies/*.toml. Higher priority wins within a tier.
  - source: extension_policy
    scope: [rules]
    merge_strategy: none
    notes: Policies contributed by extensions.
  - source: default_policy
    scope: [rules]
    merge_strategy: none
    notes: Built-in default TOML policies shipped with Gemini CLI.
  - source: workspace_policy
    scope: [rules]
    merge_strategy: none
    notes: Project-level .gemini/policies/*.toml. Currently disabled per issue #18186.

default_posture: "When nothing is configured, Gemini CLI uses general.defaultApprovalMode='default': read-only tools run automatically, write and shell tools prompt for confirmation, folder trust is enabled (schema default true, though public docs currently disagree), sandboxing is off, and MCP servers require per-server trust or confirmation."

cli_zero_permissions:
  supported: true
  invocation: gemini --approval-mode plan
  mechanism: Plan mode is the most restrictive CLI-only session posture; it denies write and shell tools while keeping read-only tools available.
  limitations: There is no CLI flag to hide or disable all built-in tools. Read-only tools (read_file, glob, grep_search, list_directory, google_web_search, etc.) still execute. To allow writes or shell commands the user must change approval mode or supply policy files. A deny-all policy file cannot be passed inline via CLI.

agent_permissions:
  allowed: true
  fm_properties:
    - subagent

yolo:
  has_interactive_yolo: true
  has_non_interactive_yolo: true
  mechanism: "--approval-mode=yolo (or deprecated --yolo/-y); can be blocked by security.disableYoloMode"

policy_engine:
  ergonomic: false
  provides_coverage: false
  gaps:
    - TOML policy tier math (Admin/User/Default priority bases) is not represented in PolicyEngine's flat rule model.
    - Regex-based argsPattern, commandRegex, and toolAnnotations matching are outside the canonical allow/ask/deny rule shape.
    - Workspace policy tier exists in Gemini CLI but is currently non-functional.
    - Sandboxing (GEMINI_SANDBOX, sandbox providers, Seatbelt profiles, toolSandboxing) is an orthogonal execution layer not modeled by PolicyEngine.
    - Folder trust safe-mode overrides policy and disables project settings, .env, MCP servers, and auto-acceptance.
    - MCP server trust, includeTools/excludeTools, and mcp.allowed/mcp.excluded lists are server-level controls beyond tool-level rules.
    - tools.core allowlisting and deprecated tools.exclude blocklisting are separate from policy rules.
    - security.disableYoloMode is an administrative lockout not expressed as a policy rule.
    - ask_user decisions become deny in non-interactive mode, which is a runtime mapping rather than a static policy effect.
    - Subagent scoping via the subagent policy field and the invoke_agent tool are not modeled.
    - Sandbox-default.toml defines OS-enforced sandbox modes (plan/default/accepting_edits) with approvedTools lists that are outside static policy.

permission_entities:
  - entity: tool
    native_names: ["toolName", "tools.core", "tools.allowed", "tools.confirmationRequired", "tools.exclude", "--allowed-tools"]
    notes: Built-in tools such as run_shell_command, read_file, write_file, replace, web_fetch, etc. tools.core is an allowlist of exposed tools; tools.exclude is deprecated.
  - entity: tool_group
    native_names: []
    notes: Gemini CLI does not expose explicit tool groups in policy; class names like ShellTool or ReadFileTool in tools.core act as coarse categories.
  - entity: command
    native_names: ["run_shell_command", "commandPrefix", "commandRegex"]
    notes: commandPrefix matches the start of the command argument; commandRegex matches a regex against the JSON-encoded arguments.
  - entity: path
    native_names: ["argsPattern", "file_path", "dir_path", "safety_checker allowed-path"]
    notes: Path constraints are expressed through regex on tool arguments or sandbox allowedPaths, not first-class path rules.
  - entity: workspace
    native_names: ["includeDirectories", "--include-directories", "tools.sandboxAllowedPaths"]
    notes: Additional directories expand where the session can read and write.
  - entity: mcp_server
    native_names: ["mcp.allowed", "mcp.excluded", "--allowed-mcp-server-names", "mcpServers"]
    notes: Server-level allowlist/blocklist and per-server trust. System settings definitions override user definitions by server name.
  - entity: mcp_tool
    native_names: ["mcpName", "toolName", "mcp_server_tool", "includeTools", "excludeTools"]
    notes: MCP tools get FQNs mcp_{serverName}_{toolName}; policy rules can target mcpName with or without toolName. Avoid underscores in server names.
  - entity: mcp_resource
    native_names: ["list_mcp_resources", "read_mcp_resource"]
    notes: Resource discovery/reading is read-only and does not prompt by default, but depends on the server being permitted to connect.
  - entity: agent
    native_names: ["invoke_agent"]
    notes: Subagents are invoked through the invoke_agent tool.
  - entity: subagent
    native_names: ["subagent"]
    notes: TOML policy rules can be scoped to a specific subagent name, or the subagent name can be used as a virtual toolName.
  - entity: mode
    native_names: ["general.defaultApprovalMode", "--approval-mode", "modes"]
    notes: Approval modes set the session baseline for approvals default, auto_edit, plan, yolo.
  - entity: approval_category
    native_names: ["allow", "ask_user", "deny"]
    notes: Fine-grained rule decisions. ask_user maps to deny in non-interactive mode.
  - entity: sandbox
    native_names: ["tools.sandbox", "security.toolSandboxing", "GEMINI_SANDBOX", "sandbox-default.toml"]
    notes: Full-process sandbox is legacy; tool-level sandboxing isolates individual tool executions. Both are separate from the policy engine.
  - entity: hook
    native_names: ["hooks.BeforeTool", "hooks.AfterTool"]
    notes: Hooks can intercept tool calls such as enter_plan_mode and exit_plan_mode.
  - entity: extension
    native_names: ["extensions", "blockGitExtensions", "allowedExtensions"]
    notes: Extensions can contribute tools, MCP servers, and policies. security.blockGitExtensions and allowedExtensions restrict extension sources.
  - entity: slash_command
    native_names: ["/permissions", "/plan", "/mcp"]
    notes: In-session commands for trust and mode management.
  - entity: unknown
    native_names: ["security.enableConseca"]
    notes: Context-aware security checker uses an LLM to dynamically generate and enforce policies.

approval_modes:
  - name: default
    effect: Read-only tools run without approval; write, shell, web_fetch, activate_skill, and MCP tool calls prompt for confirmation.
    interactive: true
    non_interactive: true
    aliases: ["default"]
  - name: auto_edit
    effect: Auto-approves write_file, replace, and web_fetch when an in-process safety checker allows the path. Shell and other mutators still prompt.
    interactive: true
    non_interactive: true
    aliases: ["auto_edit"]
  - name: plan
    effect: Read-only mode for research and design. Only read/search/planning tools are allowed; writes are limited to .md files in the plans directory. In non-interactive mode exit_plan_mode switches to yolo for implementation.
    interactive: true
    non_interactive: true
    aliases: ["plan"]
  - name: yolo
    effect: All tools are auto-approved except ask_user and plan-mode transitions. Can be blocked by security.disableYoloMode.
    interactive: true
    non_interactive: true
    aliases: ["yolo", "--yolo", "-y"]

rule_model:
  decisions: ["allow", "ask_user", "deny"]
  syntax: "TOML [[rule]] blocks with toolName, commandPrefix, commandRegex, argsPattern, mcpName, subagent, toolAnnotations, modes, interactive, priority, decision, denyMessage, and allowRedirection."
  precedence: "Highest final priority wins. Tier bases: Admin 5, User 4, Workspace 3, Extension 2, Default 1; final priority = tier_base + (priority/1000). Settings-based dynamic rules are fixed in the User tier (4.1-4.95). Within a tier, higher numeric priority wins."
  merge_semantics: "TOML rules are additive per tier; the highest-priority matching rule decides. mcpServers objects merge across settings scopes by server name. MCP includeTools arrays intersect when both sources provide lists; excludeTools arrays union; exclude always wins. tools.core replaces/limits the exposed built-in tool set."
  matcher_semantics: "toolName supports * for any tool, mcp_* for any MCP tool, and mcp_server_* for a specific server. commandPrefix matches string start. commandRegex and argsPattern are regexes tested against the JSON-encoded tool arguments. toolAnnotations requires all listed key-value pairs to be present."
  default_decision: "When no rule matches, the active approval mode decides: default asks for mutators and allows reads; plan denies most writes; yolo allows; non-interactive converts ask_user to deny."

tool_visibility:
  supported: true
  mechanisms:
    - "tools.core allowlist restricts which built-in tools are exposed to the model."
    - "tools.exclude (deprecated) removes tools from discovery."
    - "Policy deny rules without an argsPattern remove the tool from the model's memory entirely."
    - "MCP includeTools/excludeTools filter tools per server."
    - "Extensions can add or remove tools from the registry."
  notes: "There is no CLI flag to hide built-in tools directly. --allowed-tools only adds allow rules and is deprecated. A tool denied by policy without argsPattern is hidden from the model context."

sandbox:
  supported: true
  modes: ["plan", "default", "accepting_edits"]
  backends: ["macOS Seatbelt", "Docker/Podman", "Windows Native Sandbox", "gVisor/runsc (Linux)", "LXC/LXD (Linux, experimental)"]
  filesystem_control: "Full-process sandbox mounts the workspace at the same absolute path inside the container and restricts writes outside it. Tool-level sandboxing isolates individual tool executions. Additional paths can be added via SANDBOX_MOUNTS, tools.sandboxAllowedPaths, or sandbox expansion requests."
  network_control: "Seatbelt profiles choose between open/proxied network; container sandboxes can set tools.sandboxNetworkAccess. Corporate proxies can be configured via env vars or MCP server env."
  notes: "tools.sandbox is labeled legacy full-process sandboxing in the settings schema. security.toolSandboxing is the newer tool-level sandbox and defaults to false. Sandboxing applies to tool/shell execution; built-in read-only file tools run outside the sandbox boundary."

trust_and_admin:
  folder_trust: "security.folderTrust.enabled enables a trust dialog on first launch in a folder. Untrusted folders ignore project .gemini/settings.json, project .env, extension install/update/uninstall, auto-acceptance, automatic memory, MCP servers, and custom commands. The schema default is true, though public docs currently state it is disabled by default."
  managed_policy: "System-wide system-defaults.json and settings.json provide baseline and override layers. Admin-tier TOML policies can be placed in standard system directories or loaded via --admin-policy. A wrapper script can enforce GEMINI_CLI_SYSTEM_SETTINGS_PATH. These are client-side controls and can be bypassed by a determined local administrator."
  safe_mode: "An untrusted workspace runs in restricted safe mode. --skip-trust or GEMINI_CLI_TRUST_WORKSPACE=true trust the workspace for the session only."
  notes: "Local observed config (~/.gemini/trustedFolders.json) shows folder trust in use even though the user's settings.json does not explicitly enable it, supporting the schema default of true."

mcp_permissions:
  supported: true
  server_filters:
    - "mcp.allowed restricts which configured servers connect."
    - "mcp.excluded disables specific servers."
    - "--allowed-mcp-server-names restricts servers for one session."
    - "System settings definitions of mcpServers override user/project definitions by server name."
  tool_filters:
    - "Per-server includeTools allowlist and excludeTools blocklist."
    - "Policy engine rules using mcpName and/or toolName."
  trust_model: "mcpServers.<name>.trust=true bypasses confirmation for that server's tools. OAuth tokens are stored per user in ~/.gemini/mcp-oauth-tokens.json. Untrusted workspaces do not connect MCP servers."
  notes: "MCP tools run outside the tool sandbox. stdio servers are subprocesses and inherit a redacted environment unless explicitly configured. Extension-provided MCP tool lists merge restrictively: excludeTools union, includeTools intersect."

headless_behavior: In non-interactive -p mode, ask_user decisions are treated as deny, interactive trust and approval dialogs are skipped, and plan mode auto-approves enter_plan_mode/exit_plan_mode then switches to yolo for implementation. MCP servers requiring OAuth are unavailable. If folder trust is enabled and the workspace is untrusted, the CLI exits with FatalUntrustedWorkspaceError unless --skip-trust or GEMINI_CLI_TRUST_WORKSPACE is set.

approval_persistence: User choices to "Always allow" can be persisted when security.enablePermanentToolApproval is true; security.autoAddToPolicyByDefault makes this the default for low-risk tools in trusted workspaces. Folder trust and MCP server trust/OAuth tokens persist in ~/.gemini/trustedFolders.json and ~/.gemini/mcp-oauth-tokens.json.

protected_paths:
  - "gha-creds-*.json (GitHub Actions credential files are denied by sandbox-default.toml)"

security_posture: Gemini CLI's default security is a client-side static policy engine with advisory prompts, not an OS-enforced sandbox. Optional OS-enforced sandboxing (Seatbelt, containers, gVisor) can isolate tool and shell execution. Folder trust and system settings provide gating and administrative layers, but all controls are client-side and can be bypassed by a malicious actor with local administrative rights. A context-aware security checker (security.enableConseca) can add an LLM-based guardrail.

changes:
  - "Added --policy CLI flag (User-tier policy loader) alongside existing --admin-policy."
  - "Confirmed approval mode values are default, auto_edit, plan, and yolo; yolo remains CLI-only and can be blocked by security.disableYoloMode."
  - "Verified policy tier bases from source: Default 1, Extension 2, Workspace 3, User 4, Admin 5; public docs contain an internal inconsistency calling Admin base 4."
  - "Workspace policy tier (.gemini/policies/*.toml) remains disabled per issue #18186."
  - "Documented conflict between schema default (true) and public docs (disabled by default) for security.folderTrust.enabled."
  - "Separated legacy full-process sandbox (tools.sandbox) from tool-level sandboxing (security.toolSandboxing, default false)."
  - "Added newly discovered security settings: disableAlwaysAllow, enablePermanentToolApproval, autoAddToPolicyByDefault, blockGitExtensions, allowedExtensions, environmentVariableRedaction, enableConseca."
  - "Documented sandbox-default.toml sandbox modes (plan/default/accepting_edits) and approvedTools lists."
  - "Identified subagent invocation tool as invoke_agent and the subagent policy-rule scoping field."
  - "Documented MCP tool-filter merge semantics (excludeTools union, includeTools intersect) and extension override behavior."
  - "Corrected cli_zero_permissions: no CLI flag can hide all tools; plan mode is the strongest CLI-only session lockdown."
  - "Noted only gha-creds JSON is observed as a protected path in policy; no general protected-path list is documented."
  - "Added sunset context: Gemini CLI is being replaced by Antigravity CLI for unpaid tier and Google One users."

requires_claudine_update: true
reason: "Gemini CLI's permission surface combines approval modes, TOML policy tiers with fixed tier bases, regex/argsPattern rules, commandPrefix/commandRegex matching, toolAnnotations, sandbox modes and OS backends, folder trust state, MCP server filters and restrictive merge semantics, subagent scoping, settings-based dynamic rules with fixed sub-priorities, and the transition to Antigravity CLI. Claudine's PolicyEngine would need a Gemini-specific backend extension to accurately model these layers and mutate them via config/policy files."
---

# Gemini CLI Permissions

## Introduction to Gemini CLI Permissions

Gemini CLI uses a layered permission model. The highest-level knob is the **approval mode**, which selects a broad posture such as read-only or auto-approve. Under that, the **Policy Engine** evaluates TOML rules that decide whether an individual tool call is allowed, denied, or requires user confirmation. Finally, **sandboxing** and **folder trust** provide isolation and trust-gating that can override or restrict what the policy engine would otherwise permit.

Permissions can be defined through:

1. **Configuration files** — JSON `settings.json` at system, user, and project scopes, plus TOML policy files in `~/.gemini/policies/` (User tier) and system policy directories (Admin tier).
2. **Environment variables** — such as `GEMINI_SANDBOX`, `GEMINI_CLI_TRUST_WORKSPACE`, and paths that relocate config files.
3. **CLI flags** — such as `--approval-mode`, `--sandbox`, `--skip-trust`, `--policy`, and `--admin-policy`.

### Approval modes

| Mode | Behavior |
| :--- | :--- |
| `default` | Read-only tools run automatically; write and shell tools ask for confirmation. |
| `auto_edit` | Optimized for automated editing; certain write operations are auto-approved when a path safety checker passes. |
| `plan` | Read-only mode for research and design; edits are limited to `.md` plan files. In non-interactive mode, exiting plan switches to `yolo` for implementation. |
| `yolo` | All tools are auto-approved. Can only be enabled via CLI (`--approval-mode=yolo` or deprecated `--yolo`). Can be blocked with `security.disableYoloMode`. |

The active mode can be set per session with `--approval-mode` or configured as `general.defaultApprovalMode` in `settings.json`. The value `yolo` is not allowed in `general.defaultApprovalMode`; it must be requested explicitly.

### Policy Engine basics

Policy rules are written in TOML and define a decision (`allow`, `deny`, or `ask_user`) for matching tool calls. Rules can match by `toolName`, `commandPrefix`, `commandRegex`, `argsPattern`, `mcpName`, `subagent`, `toolAnnotations`, `modes`, and `interactive`. Higher-priority rules win. The tiers are:

| Tier | Base | Location |
| :--- | :--- | :--- |
| Default | 1 | Built-in policies shipped with Gemini CLI. |
| Extension | 2 | Policies defined in extensions. |
| Workspace | 3 | **Currently disabled.** `.gemini/policies/*.toml` has no effect. |
| User | 4 | `~/.gemini/policies/*.toml` |
| Admin | 5 | System directories or supplemental `--admin-policy` paths. |

Final priority is computed as `tier_base + (toml_priority / 1000)`, so Admin rules always beat User rules, which beat Extension and Default rules.

Settings-based and dynamic rules are also evaluated in the User tier with fixed sub-priorities:

| Sub-priority | Source |
| :--- | :--- |
| 4.95 | Interactive "Always Allow" choices |
| 4.9 | MCP excluded list |
| 4.4 | CLI `--allowed-tools` blocks |
| 4.3 | CLI `--allowed-tools` allows |
| 4.2 | MCP servers with `trust: true` |
| 4.1 | MCP allowed list |

### CLI parameters and precedence

The permission-related CLI parameters are listed in the frontmatter. In summary:

- `--approval-mode <mode>` sets the session approval mode.
- `--yolo` (deprecated) is an alias for `--approval-mode=yolo`.
- `--sandbox` enables sandboxed execution.
- `--skip-trust` bypasses the folder trust check.
- `--include-directories` expands the workspace.
- `--allowed-mcp-server-names` restricts which MCP servers load.
- `--allowed-tools` is a deprecated tool allowlist.
- `--policy` loads supplemental User-tier policy files.
- `--admin-policy` loads supplemental Admin-tier policy files.

Precedence is documented in the frontmatter. Key points:

- CLI flags are temporary session overrides and beat environment variables and file config.
- Environment variables beat all settings files.
- The system settings override file has the final say among settings files.
- Project settings override user settings, which override system defaults.
- For policy rules, Admin-tier rules beat User-tier rules, which beat Extension and Default-tier rules. Workspace-tier policies are currently disabled.

### Permission policy vs tool visibility

Gemini CLI separates **which tools are visible to the model** from **which visible tools are pre-approved**.

- **Approval policy** (`allow`/`ask_user`/`deny` rules, `--allowed-tools`, approval modes) decides whether a tool call runs and whether it prompts.
- **Tool visibility** (`tools.core` allowlist, `tools.exclude`, policy deny rules without `argsPattern`, MCP `includeTools`/`excludeTools`) decides which tools appear in the model's context. A tool removed by a bare deny rule never appears in the prompt, so the model cannot choose to invoke it.

For example, `tools.core = ["ReadFileTool", "GlobTool", "ShellTool(ls)"]` hides every built-in tool except the listed ones, while a policy allow rule for `run_shell_command` still leaves the shell tool visible but only auto-approves matching calls.

## Permissions Use Cases

### Default

If no environment variable, config file, or CLI switch configures permissions, Gemini CLI starts in `default` approval mode. Read-only tools such as `read_file`, `glob`, `grep_search`, and `google_web_search` run without prompting, while `write_file`, `replace`, `run_shell_command`, `web_fetch`, and `activate_skill` require confirmation. Folder trust is enabled by default according to the settings schema, so first launch in a folder prompts a trust dialog. MCP servers are discovered but their tools still require confirmation unless the server is marked `trust: true`.

A PolicyEngine description of the default posture would be:

- `can_read(path)` → Allow for files under the workspace and included directories.
- `can_write(path)` → Ask for paths in the workspace; behavior outside the workspace depends on sandbox and trust settings.
- `can_execute(command)` → Ask for `run_shell_command`.
- `can_access_domain(domain)` → Ask for `web_fetch`; `google_web_search` is allowed.
- `can_use_mcp_server(server)` / `can_use_mcp_tool(server, tool)` → Ask unless the server is trusted.
- `can_spawn_subagent(agent)` → Allow to invoke; subagent tool calls are checked individually.

This use case is only partially ergonomic in PolicyEngine. The engine can model the read/write/execute/network/MCP/agent axes, but Gemini CLI's default posture is also shaped by the active approval mode, sandbox state, and folder trust, none of which collapse cleanly into static allow/ask/deny rules.

### Whitelisting

To start with the most restrictive CLI-only posture, use `plan` mode. In `plan` mode, the CLI is read-only by default and write operations are denied except for plan `.md` files.

```bash
# Run a read-only exploration with no edits allowed
gemini --approval-mode plan "explain the auth module"

# Non-interactive read-only summary; ask_user becomes deny
gemini -p --approval-mode plan "summarize README.md"

# Allow only a specific MCP server for one session
gemini --allowed-mcp-server-names github "list my open PRs"
```

For a file-based deny-by-default configuration, create a User-tier policy:

```toml
# ~/.gemini/policies/lockdown.toml
[[rule]]
toolName = "*"
decision = "deny"
priority = 100

[[rule]]
toolName = "read_file"
decision = "allow"
priority = 200

[[rule]]
toolName = "run_shell_command"
commandPrefix = "git"
decision = "ask_user"
priority = 200
```

In interactive sessions, the `/permissions` command can change folder trust, but it cannot override a Policy Engine `deny` rule or an approval mode that forbids auto-approval.

PolicyEngine can describe this use case by setting an approval mode and adding allow rules for the approved surface. It is not fully ergonomic because Gemini CLI's strongest CLI-only lockdown (`plan`) still allows read-only tools, and there is no CLI flag to hide or disable all tools.

### YOLO

In Gemini CLI, YOLO mode is the `yolo` approval mode. A session can be put into this mode by:

- Starting with `--approval-mode=yolo`.
- Starting with the deprecated `--yolo` or `-y` flag.

Availability:

- **Interactive sessions**: yes, when started with one of the enabling flags.
- **Non-interactive sessions**: yes, `gemini -p --approval-mode=yolo` works.
- **Root/sudo on macOS and Linux**: no documented restriction. YOLO remains available to root unless an administrator disables it with `security.disableYoloMode`.

When in `yolo` mode:

- **Allowed**: almost all tool calls execute without prompting, including file edits, shell commands, web fetch, MCP tool calls, and subagent spawns. `allowRedirection` is implicitly enabled.
- **Still constrained**: sandbox boundaries, folder trust safe-mode, and `security.disableYoloMode` still apply. A `security.disableYoloMode: true` setting blocks YOLO entirely.
- **Not allowed**: `ask_user` still prompts in interactive mode, and `enter_plan_mode`/`exit_plan_mode` are denied in interactive `yolo` to avoid state conflicts.

### Root User

Gemini CLI does not document any special permission behavior when started as the root user. Unlike some other agentic CLIs, there is no published check that refuses YOLO mode or sandbox bypass based on UID. Root sessions can still use `--approval-mode=yolo` unless `security.disableYoloMode` is set, and sandboxing can still be enforced or disabled via flags and config.

### Configuring the Default

Default permissions are configured through JSON and TOML files at several scopes:

- **User scope**: `~/.gemini/settings.json` applies across all projects.
- **Repo/project scope**: `.gemini/settings.json` applies when running from that project directory.
- **Policy scope**: `~/.gemini/policies/*.toml` for User-tier rules; system policy directories for Admin-tier rules.
- **System scope**: `/etc/gemini-cli/settings.json` (Linux), `/Library/Application Support/GeminiCli/settings.json` (macOS), or `C:\ProgramData\gemini-cli\settings.json` (Windows) for machine-wide overrides.
- **System defaults scope**: the corresponding `system-defaults.json` paths provide machine-wide baselines.

Examples that illustrate the grammar:

```json
// ~/.gemini/settings.json — user-wide defaults
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

```json
// .gemini/settings.json — repo-shared defaults
{
  "tools": {
    "sandbox": "docker",
    "core": ["ReadFileTool", "GlobTool", "ShellTool(ls)"]
  },
  "mcp": {
    "allowed": ["corp-tools"]
  }
}
```

```toml
# ~/.gemini/policies/user-defaults.toml
[[rule]]
toolName = "run_shell_command"
commandPrefix = "npm"
decision = "allow"
priority = 100
modes = ["default", "auto_edit"]

[[rule]]
toolName = "write_file"
argsPattern = '"file_path":".*\\.env"'
decision = "deny"
priority = 200
denyMessage = "Writing .env files is not allowed."
```

### Extending the Base

Default permissions can be set at user scope and then narrowed or extended by narrower scopes.

**Example 1: user allows a shell command, repo denies exposure of the shell tool.**

User `~/.gemini/policies/user-defaults.toml`:

```toml
[[rule]]
toolName = "run_shell_command"
commandPrefix = "curl"
decision = "allow"
priority = 100
```

Repo `.gemini/settings.json`:

```json
{
  "tools": {
    "core": ["ReadFileTool", "GlobTool", "ShellTool(ls)"]
  }
}
```

Result: in the repository, only the allowlisted tools are available, so `curl` is effectively blocked because `run_shell_command` is no longer exposed to the model.

**Example 2: user default mode, CLI override.**

User `~/.gemini/settings.json`:

```json
{
  "general": {
    "defaultApprovalMode": "auto_edit"
  }
}
```

CLI:

```bash
gemini --approval-mode plan
```

Result: the session starts in `plan` mode. CLI flags override settings file values.

**Example 3: user denies an MCP server, project allows a specific tool.**

User `~/.gemini/policies/mcp.toml`:

```toml
[[rule]]
mcpName = "third-party-analyzer"
decision = "deny"
priority = 100
```

Repo `.gemini/settings.json`:

```json
{
  "mcpServers": {
    "third-party-analyzer": {
      "command": "/usr/local/bin/start-analyzer.sh",
      "includeTools": ["code-search"]
    }
  }
}
```

Result: the User-tier deny rule blocks all tools from the server regardless of the project `includeTools` list, because the policy rule outranks the settings-based include list.

## Tools and Permissions

Gemini CLI provides the following built-in tools. The "Default Policy" column indicates the Policy Engine decision in `default` approval mode.

| Tool | Category | Default Policy | Notes |
| :--- | :--- | :--- | :--- |
| `run_shell_command` | Execution | `ask_user` | Requires confirmation by default. |
| `glob` | File System | `allow` | Read-only search. |
| `grep_search` | File System | `allow` | Read-only search. |
| `list_directory` | File System | `allow` | Read-only directory listing. |
| `read_file` | File System | `allow` | Reads text, images, audio, and PDF. |
| `read_many_files` | File System | `allow` | Triggered by `@` file references. |
| `replace` | File System | `ask_user` | File edits require confirmation. |
| `write_file` | File System | `ask_user` | File writes require confirmation. |
| `ask_user` | Interaction | `allow` | Prompts the user for clarification. |
| `write_todos` | Interaction | `allow` | Internal task tracking. |
| `list_mcp_resources` | MCP | `allow` | Discovers MCP resources. |
| `read_mcp_resource` | MCP | `allow` | Reads MCP resources. |
| `activate_skill` | Memory | `ask_user` | Loads an agent skill. |
| `get_internal_docs` | Memory | `allow` | Retrieves CLI documentation. |
| `enter_plan_mode` | Planning | `ask_user` / `allow` | Interactive asks; non-interactive allows. |
| `exit_plan_mode` | Planning | `ask_user` / `allow` | Interactive asks; non-interactive allows. |
| `complete_task` | System | `allow` | Subagent completion tool. |
| `invoke_agent` | Subagent | `allow` | Invokes a subagent; target agent is checked individually. |
| `tracker_create_task` | Task Tracking | `allow` | Experimental task tracker. |
| `tracker_update_task` | Task Tracking | `allow` | Experimental task tracker. |
| `tracker_get_task` | Task Tracking | `allow` | Experimental task tracker. |
| `tracker_list_tasks` | Task Tracking | `allow` | Experimental task tracker. |
| `tracker_add_dependency` | Task Tracking | `allow` | Experimental task tracker. |
| `tracker_visualize` | Task Tracking | `allow` | Experimental task tracker. |
| `update_topic` | Task Tracking | `allow` | Updates session topic/status. |
| `google_web_search` | Web | `allow` | Google Search is allowed by default. |
| `web_fetch` | Web | `ask_user` | Fetching arbitrary URLs requires confirmation. |

Permissions map to tool calls through the Policy Engine. Rules can target built-in tool names, MCP tool FQNs (`mcp_{serverName}_{toolName}`), or subagent names via the `invoke_agent` tool. In `plan` mode, write tools are heavily restricted; in `yolo` mode, a high-priority rule allows all tools.

## Sandboxing, Trust, and Administrative Controls

### Sandboxing

Gemini CLI's sandbox is a separate layer from permission modes and rules. It provides OS-level isolation for tool and shell execution.

- **Backends**: macOS uses Seatbelt (`sandbox-exec`); Linux/Windows use Docker/Podman; Windows also supports Windows Native Sandbox; Linux supports gVisor/runsc and LXC/LXD (experimental).
- **Modes**: The internal `sandbox-default.toml` defines three sandbox modes that loosely mirror approval modes: `plan` (readonly, no network), `default` (writes restricted to workspace, network off), and `accepting_edits` (additional approved tools like `sed`, `awk`, `perl`).
- **Full-process sandbox**: configured via `--sandbox`, `GEMINI_SANDBOX`, or `tools.sandbox`. The schema labels this as the legacy sandbox.
- **Tool-level sandbox**: configured via `security.toolSandboxing` (default `false`). This isolates individual tool executions instead of the entire CLI process.
- **Filesystem**: by default, sandboxed commands can write only to the workspace. Use `SANDBOX_MOUNTS`, `tools.sandboxAllowedPaths`, or sandbox expansion requests to widen access.
- **Network**: controlled by Seatbelt profiles or `tools.sandboxNetworkAccess`. Corporate proxies can be configured via environment variables.
- **Scope**: sandboxing applies to tool/shell execution. Built-in read-only file tools run outside this boundary.

Permissions and sandboxing are complementary:

- Permission rules block Gemini from attempting restricted actions.
- Sandbox restrictions prevent shell/tool commands from reaching resources outside defined boundaries, even if a prompt injection bypasses the policy engine.

### Trust and administrative controls

**Folder/project trust**: first-time launches in a folder prompt a trust dialog when `security.folderTrust.enabled` is true. The schema default is `true`, though public documentation currently states it is disabled by default. Untrusted folders ignore project `settings.json`, `.env` files, extension install/update/uninstall, tool auto-acceptance, automatic memory, MCP servers, and custom commands. Trust is saved in `~/.gemini/trustedFolders.json`.

**Managed/admin policy**: system-wide `system-defaults.json` and `settings.json` provide baseline and override layers. Admin-tier TOML policies can be placed in standard system directories or loaded via `--admin-policy`. These supplemental policies are ignored if any `.toml` files exist in the standard system location, preventing flag-based bypass when central policy is established. A wrapper script can enforce `GEMINI_CLI_SYSTEM_SETTINGS_PATH`. The enterprise docs explicitly note these are client-side controls and can be bypassed by a determined local administrator.

**Safe mode**: an untrusted workspace runs in restricted safe mode. `--skip-trust` or `GEMINI_CLI_TRUST_WORKSPACE=true` trust the workspace for the session only.

### Protected paths

The only provider-reserved path pattern observed in the built-in sandbox policy is GitHub Actions credential files:

- `gha-creds-*.json` — denied by `sandbox-default.toml`.

No general list of protected paths (such as `.git`, shell config, or `.env`) is documented or observed in the current policy files.

## MCP and Permissions

MCP servers extend Gemini CLI with external tools. Their configuration lives in the `mcpServers` object of `settings.json` and is governed by several permission layers.

Permission controls for MCP:

- **Server allowlist**: the global `mcp.allowed` array restricts which configured servers connect. If it is set, servers not in the list are ignored.
- **Server blocklist**: the global `mcp.excluded` array disables specific servers.
- **Per-server trust**: `mcpServers.<name>.trust: true` bypasses confirmation for all tools from that server.
- **Tool filtering**: `includeTools` exposes only listed tools; `excludeTools` removes listed tools and takes precedence over `includeTools`. When merging extension and local configs, `excludeTools` arrays are unioned and `includeTools` arrays are intersected.
- **Environment redaction**: sensitive host environment variables are automatically redacted from MCP server processes unless explicitly listed in the server's `env` block.
- **Policy Engine rules**: use `mcpName` to target a server, optionally combined with `toolName`, or `mcpName = "*"` for all servers.
- **Folder trust**: when a workspace is untrusted, MCP servers do not connect at all, regardless of policy.

MCP tools are registered with fully qualified names of the form `mcp_{serverName}_{toolName}`. Avoid underscores in server names because the policy parser splits the FQN on the first underscore after `mcp_`.

To make MCP safer:

- Define approved servers in system `settings.json` and list them in `mcp.allowed`.
- Use `includeTools` to expose only the tools a workflow needs.
- Set `trust: false` by default and rely on Policy Engine rules or per-session approval.
- Run in a sandbox so MCP server side effects are isolated from the host.
- In untrusted workspaces, folder trust automatically disables MCP connections.

## Non-Interactive Behavior

In non-interactive `-p` mode, Gemini CLI cannot show interactive permission prompts. Key behaviors:

- `ask_user` decisions are treated as `deny`.
- Plan mode auto-approves `enter_plan_mode` and `exit_plan_mode`, then switches to `yolo` for implementation.
- OAuth-enabled MCP servers are unavailable because they require a browser flow.
- Security approval dialogs and folder trust dialogs are skipped. If folder trust is enabled and the workspace is untrusted, the CLI exits with `FatalUntrustedWorkspaceError` unless `--skip-trust` or `GEMINI_CLI_TRUST_WORKSPACE=true` is set.

To avoid hangs, use `--approval-mode plan`, `--approval-mode yolo`, policy files, or restrict the tool surface via `tools.core` in settings.

## Sources

- [Gemini CLI homepage](https://geminicli.com/)
- [Gemini CLI configuration reference](https://geminicli.com/docs/reference/configuration/)
- [Policy engine reference](https://geminicli.com/docs/reference/policy-engine/)
- [Tools reference](https://geminicli.com/docs/reference/tools/)
- [Trusted folders](https://geminicli.com/docs/cli/trusted-folders/)
- [Sandboxing](https://geminicli.com/docs/cli/sandbox/)
- [Plan mode](https://geminicli.com/docs/cli/plan-mode/)
- [Headless mode](https://geminicli.com/docs/cli/headless/)
- [MCP servers](https://geminicli.com/docs/tools/mcp-server/)
- [Enterprise configuration](https://geminicli.com/docs/cli/enterprise/)
- [Gemini CLI GitHub repository](https://github.com/google-gemini/gemini-cli)
- [settings.schema.json](https://raw.githubusercontent.com/google-gemini/gemini-cli/main/schemas/settings.schema.json)
- [packages/core/src/policy/policies/read-only.toml](https://github.com/google-gemini/gemini-cli/blob/main/packages/core/src/policy/policies/read-only.toml)
- [packages/core/src/policy/policies/write.toml](https://github.com/google-gemini/gemini-cli/blob/main/packages/core/src/policy/policies/write.toml)
- [packages/core/src/policy/policies/plan.toml](https://github.com/google-gemini/gemini-cli/blob/main/packages/core/src/policy/policies/plan.toml)
- [packages/core/src/policy/policies/yolo.toml](https://github.com/google-gemini/gemini-cli/blob/main/packages/core/src/policy/policies/yolo.toml)
- [packages/core/src/policy/policies/sandbox-default.toml](https://github.com/google-gemini/gemini-cli/blob/main/packages/core/src/policy/policies/sandbox-default.toml)
- [packages/core/src/policy/policies/agents.toml](https://github.com/google-gemini/gemini-cli/blob/main/packages/core/src/policy/policies/agents.toml)

## Changelog

- 2026-07-02: Refreshed research against Gemini CLI v0.46.0, current documentation, source policy files, and settings schema. Added `--policy`, separated legacy and tool-level sandboxing, documented folder trust default conflict, added newly discovered security settings, corrected subagent invocation and MCP merge semantics, and noted the Gemini CLI to Antigravity CLI transition.
