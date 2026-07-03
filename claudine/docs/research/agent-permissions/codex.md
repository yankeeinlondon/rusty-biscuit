---
$schema: ./_schema.yaml
created: 2026-07-01
last_updated: 2026-07-02
agent: open_code
model: kimi-for-coding/k2p7

cli_params:
  - param: sandbox
    style: switch
    description: "Select the sandbox policy for model-generated shell commands."
    example: "codex --sandbox workspace-write"
    example_description: "Allows Codex to read and write inside the workspace and prompts for actions outside that scope."
  - param: ask-for-approval
    style: switch
    description: "Control when Codex pauses for human approval before running a command. Values are untrusted, on-request, and never; on-failure is deprecated."
    example: "codex --ask-for-approval on-request"
    example_description: "Codex asks before executing actions that leave the sandbox or use the network, but runs workspace-safe commands automatically."
  - param: dangerously-bypass-approvals-and-sandbox
    style: switch
    description: "Run every command without approvals or sandboxing. Alias --yolo. Only use inside an externally hardened environment."
    example: "codex --yolo"
    example_description: "Disables both the sandbox and approval prompts for the session."
  - param: add-dir
    style: switch
    description: "Grant additional directories write access alongside the main workspace. Repeatable for multiple paths."
    example: "codex --add-dir ../shared --add-dir ../docs"
    example_description: "Expands the writable workspace to include sibling directories for this session."
  - param: config
    style: switch
    description: "Override configuration values at runtime. Values parse as TOML if possible; otherwise the literal string is used. Can override permission-related keys."
    example: "codex -c 'default_permissions = \":read-only\"'"
    example_description: "Sets the active permission profile for the session without editing config files."
  - param: profile
    style: switch
    description: "Layer a profile config file from $CODEX_HOME on top of the base user config. Useful for saved permission presets."
    example: "codex --profile readonly-quiet"
    example_description: "Loads ~/.codex/readonly-quiet.config.toml, which might set a read-only profile and approval never."
  - param: ignore-user-config
    style: switch
    description: "Skip loading the user's ~/.codex/config.toml for this run, useful for controlled automation environments."
    example: "codex exec --ignore-user-config --sandbox read-only"
    example_description: "Runs a non-interactive task without any user-level permission defaults."
  - param: ignore-rules
    style: switch
    description: "Skip user and project execpolicy .rules files for this run."
    example: "codex exec --ignore-rules --sandbox workspace-write"
    example_description: "Runs without applying any command prefix rules from ~/.codex/rules/ or .codex/rules/."
  - param: search
    style: switch
    description: "Enable live web search. Sets web_search to live instead of the default cached mode."
    example: "codex --search"
    example_description: "Allows the model to fetch live web pages rather than using the cached web search index."
  - param: dangerously-bypass-hook-trust
    style: switch
    description: "Run enabled hooks without requiring persisted hook trust for this invocation. Intended only for automation that already vets hook sources."
    example: "codex --dangerously-bypass-hook-trust"
    example_description: "Bypasses the trust gate for lifecycle hooks for this session."
  - param: enable
    style: switch
    description: "Enable a feature flag. Equivalent to -c features.<name>=true. Repeatable."
    example: "codex --enable network_proxy"
    example_description: "Turns on the experimental network_proxy feature for this session."
  - param: disable
    style: switch
    description: "Disable a feature flag. Equivalent to -c features.<name>=false. Repeatable."
    example: "codex --disable shell_tool"
    example_description: "Removes the shell tool from the session by disabling its feature flag."

env_vars:
  - name: CODEX_HOME
    effect: "Sets the root directory for Codex state, including config.toml, auth, logs, sessions, skills, rules, and standalone package metadata. Defaults to ~/.codex."
  - name: CODEX_SQLITE_HOME
    effect: "Sets where SQLite-backed state is stored. The sqlite_home config option takes precedence, and relative paths resolve from the current working directory."

config_files:
  - os: all
    user: ~/.codex/config.toml
    repo: .codex/config.toml
    notes: "Project-scoped files load only for trusted projects. On Unix there is also a system config at /etc/codex/config.toml and managed requirements at /etc/codex/requirements.toml; on Windows managed requirements live under %ProgramData%/OpenAI/Codex/requirements.toml and managed defaults under ~/.codex/managed_config.toml (non-Unix) or /etc/codex/managed_config.toml (Unix). Profile files live at $CODEX_HOME/<name>.config.toml."

precedence:
  - source: managed_requirements
    scope: [approval_mode, sandbox, rules, mcp, permission_profiles, network, feature_flags]
    merge_strategy: none
    notes: "Cloud-managed (ChatGPT Business/Enterprise), macOS MDM com.openai.codex requirements_toml_base64, and system /etc/codex/requirements.toml enforce constraints. Conflicting user/repo/CLI values fall back to an allowed value."
  - source: managed_defaults
    scope: [all_defaults]
    merge_strategy: none
    notes: "System /etc/codex/managed_config.toml and macOS MDM config_toml_base64 provide starting values that override CLI --config and user config. Users can change them mid-session, but managed defaults are reapplied on next launch."
  - source: cli
    scope: [sandbox_mode, approval_policy, default_permissions, add_dir, network_proxy, web_search, ignore_user_config, ignore_rules, profile]
    merge_strategy: none
    notes: "CLI flags and -c overrides are temporary session overrides. They cannot override managed requirements or managed defaults."
  - source: project_config
    scope: [permissions, sandbox, rules, mcp, hooks, agents]
    merge_strategy: nearest
    notes: ".codex/config.toml and .codex/rules/ load only for trusted projects. When multiple project layers exist, the file closest to the current working directory wins."
  - source: profile
    scope: [permissions, sandbox, approval_policy, mcp]
    merge_strategy: shallow
    notes: "$CODEX_HOME/<name>.config.toml selected with --profile layers on top of the base user config; scalars replace, permission-profile tables merge shallowly."
  - source: user_config
    scope: [permissions, sandbox, rules, mcp, hooks]
    merge_strategy: none
    notes: "~/.codex/config.toml and ~/.codex/rules/ provide the user's baseline. Observed local config includes project trust_level entries and feature flags."
  - source: system_config
    scope: [defaults]
    merge_strategy: none
    notes: "/etc/codex/config.toml on Unix provides machine-wide defaults below user config."
  - source: built_in_defaults
    scope: [all]
    merge_strategy: none
    notes: "VCS-aware default: workspace-write + on-request for version-controlled/trusted folders, read-only + on-request for non-VCS or untrusted folders. Network is off by default."

default_posture: "When nothing is configured, Codex detects the working directory. Version-controlled/trusted folders start in an Auto posture (workspace-write sandbox with on-request approvals). Non-version-controlled or untrusted folders start in read-only sandbox with on-request approvals. Network access is off by default."

cli_zero_permissions:
  supported: true
  invocation: "codex exec --ignore-user-config --sandbox read-only --ask-for-approval never"
  mechanism: "Starts in the read-only sandbox with approval policy set to never, so any command or write that would require approval is denied rather than prompted. Additional permissions can be added back via --sandbox, --ask-for-approval, --add-dir, or -c overrides."
  limitations: "Reads inside the workspace and temp directories are still allowed because Codex has no native no-read mode. There is no CLI flag to hide individual built-in tools; disabling feature flags such as features.shell_tool is a separate step."

agent_permissions:
  allowed: true
  fm_properties:
    - sandbox_mode
    - default_permissions
    - approval_policy
    - mcp_servers
    - skills.config
    - model
    - model_reasoning_effort

yolo:
  has_interactive_yolo: true
  has_non_interactive_yolo: true
  mechanism: "--dangerously-bypass-approvals-and-sandbox or --yolo CLI flag; equivalent to sandbox_mode = danger-full-access with approval_policy = never. Also reachable interactively via /permissions."

policy_engine:
  ergonomic: false
  provides_coverage: false
  gaps:
    - "Dual-layer model: sandbox mode (read-only/workspace-write/danger-full-access or beta permission profiles) and approval policy are orthogonal, not a unified allow/ask/deny rule surface."
    - "Beta permission profiles use filesystem tokens such as :minimal, :workspace_roots, :root, :tmpdir, and :slash_tmp, plus network domain rules that do not map directly to PolicyEngine path/network queries."
    - "features.network_proxy and sandbox_workspace_write.network_access interplay is not represented in PolicyEngine's command/network axes."
    - "execpolicy .rules files use Starlark prefix_rule with allow/prompt/forbidden decisions outside PolicyEngine's static rule model."
    - "Managed requirements (requirements.toml) enforce constraints and defaults from cloud, MDM, and system sources with higher precedence than user configuration."
    - "Granular approval_policy exposes per-category toggles: sandbox_approval, rules, mcp_elicitations, request_permissions, and skill_approval."
    - "Custom agent TOML files can override sandbox_mode, default_permissions, approval_policy, and mcp_servers independently of the parent session."
    - "No built-in tool-visibility flag exists; feature flags and app/MCP tool lists are separate control surfaces."

permission_entities:
  - entity: tool
    native_names: ["shell", "apply_patch", "web_search", "image_generation", "features.shell_tool", "features.web_search"]
    notes: "Built-in tools are gated by sandbox mode and approval policy; no per-tool allow/deny list exists outside disabling whole feature flags."
  - entity: tool_group
    native_names: ["features.shell_tool", "features.web_search"]
    notes: "Feature flags can disable whole tool categories."
  - entity: command
    native_names: ["execpolicy .rules prefix_rule", "approval_policy untrusted"]
    notes: "Command prefix rules decide whether a command runs outside the sandbox; the untrusted approval policy allows only a trusted command set without prompting."
  - entity: path
    native_names: ["permissions.<name>.filesystem", "sandbox_mode", "--add-dir", "deny_read requirements"]
    notes: "Filesystem access is governed by sandbox mode or beta permission profile path rules (read/write/deny) and protected-path carveouts."
  - entity: workspace
    native_names: ["workspace_roots", "--add-dir", ":workspace_roots"]
    notes: "Workspace roots define where read/write rules apply; --add-dir adds extra writable directories for the session."
  - entity: mcp_server
    native_names: ["mcp_servers.<id>", "mcp_servers.<id>.enabled", "mcp_servers.<id>.required", "plugins.<plugin>.mcp_servers.<server>"]
    notes: "MCP servers can be enabled, required, and filtered by managed identity rules."
  - entity: mcp_tool
    native_names: ["mcp_servers.<id>.enabled_tools", "mcp_servers.<id>.disabled_tools", "mcp_servers.<id>.default_tools_approval_mode", "mcp_servers.<id>.tools.<tool>.approval_mode"]
    notes: "Per-tool allow/deny and approval mode within a server."
  - entity: mcp_resource
    native_names: ["MCP resources exposed by permitted servers"]
    notes: "Resource access depends on the server being permitted; no separate resource permission layer is documented."
  - entity: agent
    native_names: ["agents.<name>.config_file", "spawn_agent", "agents.max_depth", "agents.max_threads"]
    notes: "Built-in and custom agents inherit parent policy unless overridden by their config file."
  - entity: subagent
    native_names: ["custom agent TOML files"]
    notes: "Custom agent files under ~/.codex/agents/ or .codex/agents/ can override sandbox_mode, default_permissions, approval_policy, mcp_servers, etc."
  - entity: mode
    native_names: ["sandbox_mode", "default_permissions", "--sandbox"]
    notes: "Sandbox mode or permission profile selects the coarse filesystem/network boundary."
  - entity: approval_category
    native_names: ["approval_policy", "approval_policy.granular"]
    notes: "Approval policy decides when prompts surface; granular policy toggles categories such as sandbox_approval, rules, mcp_elicitations, request_permissions, and skill_approval."
  - entity: sandbox
    native_names: ["sandbox_mode", "default_permissions", "features.network_proxy", "[windows].sandbox"]
    notes: "OS-enforced boundary for spawned commands."
  - entity: hook
    native_names: ["hooks.PreToolUse", "hooks.PermissionRequest", "allow_managed_hooks_only"]
    notes: "Lifecycle hooks can intercept tool/permission requests; managed hooks can be enforced exclusively."
  - entity: extension
    native_names: ["plugins.<plugin>.mcp_servers", "marketplaces.allowed_sources"]
    notes: "Plugins can bundle MCP servers and tools; managed policy can restrict marketplace sources."
  - entity: slash_command
    native_names: ["/permissions", "/status", "/agent"]
    notes: "In-session slash commands can change mode, view workspace boundaries, and manage subagents."

approval_modes:
  - name: untrusted
    effect: "Only known-safe read/trusted commands run automatically; mutating or external commands prompt."
    interactive: true
    non_interactive: true
    aliases: ["untrusted", "-a untrusted", "--ask-for-approval untrusted"]
  - name: on-request
    effect: "Workspace-safe actions run automatically; sandbox escalations, network use, and external edits prompt."
    interactive: true
    non_interactive: true
    aliases: ["on-request", "-a on-request", "--ask-for-approval on-request"]
  - name: never
    effect: "Never ask for approval; actions that cannot proceed without approval fail or are denied."
    interactive: true
    non_interactive: true
    aliases: ["never", "-a never", "--ask-for-approval never"]
  - name: granular
    effect: "Per-category toggles for sandbox_approval, rules, mcp_elicitations, request_permissions, and skill_approval; false categories auto-reject."
    interactive: true
    non_interactive: true
    aliases: ["granular", "approval_policy = { granular = {...} }"]
  - name: bypassPermissions
    effect: "Skips all confirmation prompts and runs commands without sandboxing."
    interactive: true
    non_interactive: true
    aliases: ["--dangerously-bypass-approvals-and-sandbox", "--yolo"]
  - name: auto_review
    effect: "Eligible approval requests are reviewed by an automatic reviewer agent before running."
    interactive: true
    non_interactive: false
    aliases: ["approvals_reviewer = auto_review"]

rule_model:
  decisions: ["allow", "prompt", "forbidden"]
  syntax: "Starlark prefix_rule(pattern=[...], decision='allow'|'prompt'|'forbidden', justification='...', match=[...], not_match=[...]). Pattern elements are literal strings or unions of literals."
  precedence: "Most restrictive decision wins when multiple rules match: forbidden > prompt > allow."
  merge_semantics: "Rules from user and project .rules files merge; the most restrictive decision across matching rules wins. Managed requirements rules also merge and are limited to prompt or forbidden."
  matcher_semantics: "Exact prefix match against the command's argument list; bash -lc / bash -c / zsh/sh equivalents are split into separate commands when the script is a safe linear chain; union alternatives match one of several literals at a position."
  default_decision: "When no execpolicy rule matches, fall back to the active sandbox mode and approval policy."

tool_visibility:
  supported: true
  mechanisms:
    - "Feature flags (e.g. features.shell_tool, features.web_search) can disable whole built-in tool categories."
    - "web_search = 'disabled' removes the web search tool."
    - "MCP/app servers expose only tools in enabled_tools and hide tools in disabled_tools."
    - "Managed requirements can disable feature surfaces such as browser_use, computer_use, and in_app_browser."
  notes: "There is no CLI flag to restrict the built-in tool surface per tool; --tools does not exist. Denied MCP/app tools are hidden from the model context."

sandbox:
  supported: true
  modes: ["read-only", "workspace-write", "danger-full-access", ":read-only", ":workspace", ":danger-full-access"]
  backends: ["macOS Seatbelt / sandbox-exec", "Linux/WSL2 bubblewrap + seccomp (Landlock fallback)", "native Windows sandbox (elevated/unelevated)"]
  filesystem_control: "Sandbox mode selects coarse read/write roots; workspace-write protects .git/.codex/.agents as read-only; --add-dir adds writable roots; beta permission profiles provide fine-grained path rules with tokens; managed deny_read requirements enforce additional read blocks."
  network_control: "Network is off by default; enable with sandbox_workspace_write.network_access=true; constrain with features.network_proxy domains/Unix sockets; local/private guard; managed experimental_network requirements."
  notes: "Applies to spawned commands. Unsupported policies are refused rather than run unsandboxed. Built-in file operations and MCP/app tools have their own controls."

trust_and_admin:
  folder_trust: "Project .codex/config.toml, .codex/rules/, .codex/agents/, project MCP servers, hooks, and skills load only for trusted projects. Trust is saved per directory; untrusted projects fall back to user/system config."
  managed_policy: "Cloud-managed requirements (ChatGPT Business/Enterprise), macOS MDM com.openai.codex requirements_toml_base64, and system /etc/codex/requirements.toml enforce constraints. /etc/codex/managed_config.toml or MDM config_toml_base64 provides managed defaults that override CLI --config and user config. Allowed lists, deny_read, experimental_network, managed hooks, feature pins, and marketplace restrictions are supported."
  safe_mode: "No single --safe-mode flag. Equivalent hardening can be achieved by disabling feature flags (hooks, multi_agent, shell_tool, web_search, browser_use, computer_use, in_app_browser) and ignoring project config/rules."
  notes: "Managed requirements take precedence over all user/repo/CLI sources; managed defaults take precedence over CLI --config."

mcp_permissions:
  supported: true
  server_filters:
    - "mcp_servers.<id>.enabled and .required"
    - "mcp_servers.<id>.enabled_tools / disabled_tools"
    - "managed requirements mcp_servers approved identity list (command/url/args)"
    - "plugins.<plugin>.mcp_servers.<server> controls"
    - "Empty mcp_servers table disables all MCP servers"
  tool_filters:
    - "mcp_servers.<id>.default_tools_approval_mode (auto/prompt/approve)"
    - "mcp_servers.<id>.tools.<tool>.approval_mode"
  trust_model: "Project-scoped MCP servers require a trusted project. OAuth credentials stored per user. stdio servers inherit limited environment via env_vars; remote HTTP servers make network requests outside the sandbox."
  notes: "MCP tools run outside the Bash sandbox. stdio server environment can be constrained with env_vars; no response interception/sanitization layer is documented."

headless_behavior: "In codex exec, interactive approval prompts cannot be shown; use --ask-for-approval never with an appropriate sandbox, or --sandbox danger-full-access, to avoid hangs. Actions that still require approval fail and are reported back to the model; required MCP servers that fail to initialize cause codex exec to exit with an error."

approval_persistence: "Allow-list additions made in the TUI are written to ~/.codex/rules/default.rules and persist across sessions. Sandbox mode or approval policy changes made with /permissions are runtime/session-scoped; managed defaults are reapplied on the next launch."

protected_paths:
  - "<writable_root>/.git"
  - "<writable_root>/.codex"
  - "<writable_root>/.agents"
  - "resolved Git directory from a .git pointer file"

security_posture: "Codex CLI's security is a combination of OS-enforced sandboxes for spawned commands (Seatbelt on macOS, bubblewrap/seccomp on Linux/WSL2, native sandbox on Windows), an approval-policy UX guardrail, Starlark-based execpolicy rules, and managed requirements/defaults from enterprise policy. The sandbox is the strongest technical boundary; approvals and rules are client-side policy layers."

changes:
  - "Confirmed current CLI version is 0.142.5; legacy codex exec --full-auto is deprecated in favor of explicit --sandbox flags."
  - "Added --dangerously-bypass-hook-trust, --enable, --disable, and --search to the permission-related CLI surface."
  - "Approval policy on-failure is now deprecated; granular approval policy with per-category toggles is documented."
  - "Updated default posture: version-controlled/trusted folders default to workspace-write + on-request; non-VCS or untrusted folders default to read-only."
  - "Expanded sandbox coverage: native Windows elevated/unelevated sandbox, WSL2 Linux sandbox, AppArmor/bwrap notes, and refusal to run unsupported policies."
  - "Added beta permission profiles with [permissions.<name>], workspace_roots, filesystem tokens, and network domain rules; documented incompatibility with legacy sandbox_mode."
  - "Documented execpolicy .rules Starlark prefix rules, codex execpolicy check, and safe command-splitting behavior."
  - "Documented managed requirements (requirements.toml) and managed defaults (managed_config.toml/MDM), including allowed lists, deny_read, experimental_network, managed hooks, and marketplace restrictions."
  - "Documented Auto-review (approvals_reviewer = auto_review) as an alternative approval reviewer."
  - "Documented custom agent/subagent permission overrides and .codex/agents/ config files."
  - "Updated MCP permission controls: plugin-bundled MCP servers, managed identity matching, and environment forwarding via env_vars."
  - "Updated protected paths to include .agents and resolved gitdir from pointer files."

requires_claudine_update: true
reason: "Codex CLI uses a dual-layer permission model, beta permission profiles, execpolicy Starlark rules, managed requirements/defaults, granular approval categories, and custom agent overrides. Claudine's PolicyEngine currently models a single canonical allow/ask/deny surface; supporting Codex accurately requires backend extensions for sandbox/approval axes, permission profile mapping, rule evaluation, and managed policy precedence."
---

# Codex CLI Permissions

## Introduction to Codex CLI Permissions

Codex CLI uses two independent but complementary layers:

- **Sandbox mode / permission profile**: defines what model-generated commands can technically reach (which files they can read or write, and whether they can use the network).
- **Approval policy**: defines when Codex must pause and ask before executing an action.

Permissions can be defined through:

1. **Configuration files** in TOML, primarily `~/.codex/config.toml` for user defaults and `.codex/config.toml` for project-scoped overrides. Project layers load only for trusted projects.
2. **CLI flags** such as `--sandbox`, `--ask-for-approval`, and `--yolo`.
3. **In-session controls** such as `/permissions` in the interactive TUI.

### Sandbox modes

| Mode | Filesystem | Network | Best for |
| :--- | :--- | :--- | :--- |
| `read-only` | Read workspace and temp directories only | Off | Exploration, safe browsing, CI read tasks |
| `workspace-write` | Read and write workspace and temp directories; `.git`, `.codex`, and `.agents` stay read-only | Off by default; enable with `sandbox_workspace_write.network_access` | Everyday coding in a trusted repo |
| `danger-full-access` | No sandbox restrictions | Unrestricted | Isolated containers, CI runners where the outer environment is the security boundary |

### Permission profiles (beta)

Codex also supports named **permission profiles** that combine filesystem rules and network rules in a single `[permissions.<name>]` table. Built-in profiles are `:read-only`, `:workspace`, and `:danger-full-access`. Permission profiles do **not** compose with the older `sandbox_mode` / `sandbox_workspace_write` settings; configure one system or the other. If `sandbox_mode` appears in any loaded config file, on the CLI, or in a selected profile, Codex falls back to the older sandbox system.

### Approval policies

| Policy | Behavior |
| :--- | :--- |
| `untrusted` | Only known-safe read/trusted commands run automatically; mutating or external commands require approval |
| `on-request` | Workspace-safe actions run automatically; sandbox escalations, network use, and external edits prompt |
| `never` | No approval prompts; actions that cannot proceed without approval fail or are denied |
| `granular` | Per-category toggles for `sandbox_approval`, `rules`, `mcp_elicitations`, `request_permissions`, and `skill_approval` |

Deprecated `on-failure` is still accepted but maps to `on-request` behavior.

### CLI parameters and precedence

The permission-related CLI parameters are listed in the frontmatter. In summary:

- `--sandbox <mode>` sets the session sandbox policy.
- `--ask-for-approval <policy>` sets when Codex must pause for approval.
- `--dangerously-bypass-approvals-and-sandbox` (alias `--yolo`) disables both the sandbox and approvals.
- `--add-dir <path>` expands the writable workspace.
- `-c key=value` or `--config key=value` overrides any config value, including permission profiles and feature flags.
- `--profile <name>` layers a saved preset from `~/.codex/profile-name.config.toml`.
- `--ignore-user-config` and `--ignore-rules` strip user-level config and execpolicy rules for controlled runs.
- `--enable` / `--disable` toggle feature flags such as `network_proxy` or `shell_tool`.
- `--search` enables live web search.
- `--dangerously-bypass-hook-trust` bypasses the trust gate for lifecycle hooks.

Precedence is documented in the frontmatter. The key points are:

- Managed requirements from cloud, MDM, or `/etc/codex/requirements.toml` constrain values and cannot be overridden.
- Managed defaults from `/etc/codex/managed_config.toml` or MDM override CLI `--config` and user config at startup.
- CLI flags and `--config` overrides beat all file-based config except managed layers.
- Project `.codex/config.toml` overrides user config for trusted projects; the closest file to the current working directory wins when multiple project layers exist.
- Profile files layer between project config and user config.
- System `/etc/codex/config.toml` provides defaults below user config.

### Permission policy vs tool visibility

Codex does **not** have a single built-in tool-visibility flag like `--tools`. Instead, visibility and approval are controlled separately:

- **Approval policy** (`approval_policy`, `--ask-for-approval`, granular toggles) decides whether a tool call runs and whether it prompts.
- **Tool visibility** is achieved through feature flags (e.g. `features.shell_tool = false`, `web_search = "disabled"`) and MCP/app tool allowlists (`enabled_tools` / `disabled_tools`). A disabled feature removes the tool from the model's context entirely, while an enabled but unapproved tool may still prompt depending on the policy.

## Permissions Use Cases

### Default

If no environment variable, config file, or CLI switch configures permissions, Codex inspects the working directory. For trusted version-controlled folders, the effective default is `sandbox_mode = "workspace-write"` with `approval_policy = "on-request"`: Codex can read, edit, and run commands inside the workspace, but must ask before editing outside the workspace or using the network. For untrusted or non-version-controlled folders, Codex starts in `read-only` with `on-request` approvals. Network access is off by default.

A PolicyEngine description of the default posture would need to represent:

- `can_read(path)` → Allow for workspace and temp paths; Ask for paths outside the workspace.
- `can_write(path)` → Allow for workspace paths (except protected subpaths); Ask or Deny for paths outside the workspace.
- `can_execute(command)` → Allow for workspace-safe commands; Ask for commands that leave the workspace or use the network.
- `can_access_domain(domain)` → Deny by default; Ask if network access is enabled.
- `can_spawn_subagent(agent)` → Allow, but the subagent inherits the same sandbox and approval policy unless its custom agent file overrides them.

This use case is only partially ergonomic in PolicyEngine. The engine can model read/write/execute/network/agent axes, but Codex's default posture depends on VCS status and trust, and the Ask/Allow split is driven by sandbox mode rather than explicit rules. Without changes, PolicyEngine cannot express the dynamic trust-gated default or the protected-path carveouts that Codex applies inside `workspace-write`.

### Whitelisting

To start with the narrowest practical baseline and require every needed permission to be asked for or explicitly declared, use the read-only sandbox with `untrusted` or `never` approval policy, and avoid enabling network access.

CLI examples:

```bash
# Non-interactive read-only CI task; never prompts, writes and commands are denied
codex exec --ignore-user-config --sandbox read-only --ask-for-approval never "summarize the repo"

# Interactive exploration that asks before any action outside the trusted read set
codex --sandbox read-only --ask-for-approval untrusted "explain the auth module"

# Workspace-write for edits, but still ask before network or external commands
codex --sandbox workspace-write --ask-for-approval on-request "refactor the parser"
```

To grant additional permissions for one session, use `--add-dir`, `-c features.network_proxy.enabled=true`, or `-c sandbox_workspace_write.network_access=true`.

The best **CLI-only, session-scoped** way to start from a locked-down posture is documented in the frontmatter's `cli_zero_permissions` field:

```bash
codex exec --ignore-user-config --sandbox read-only --ask-for-approval never
```

This denies mutating commands and network use without changing persisted config. You can then add back only what is needed via CLI flags.

PolicyEngine can describe the whitelisted posture by setting a restrictive approval mode and adding allow rules for the approved read surface. However, Codex does not have a single "deny all by default" mode that maps cleanly to PolicyEngine's `Deny` default. The closest equivalent is `read-only` + `never`, which still allows reads without per-file prompts. PolicyEngine cannot force Codex to prompt for every read; the approval prompt behavior is a runtime UI concern tied to sandbox mode and approval policy, not a static rule.

### YOLO

In Codex CLI, YOLO mode is activated by `--dangerously-bypass-approvals-and-sandbox` or its alias `--yolo`. It is also equivalent to setting `sandbox_mode = "danger-full-access"` together with `approval_policy = "never"`.

Ways to enter YOLO mode:

- Start with `--yolo` or `--dangerously-bypass-approvals-and-sandbox`.
- Start with `--sandbox danger-full-access --ask-for-approval never`.
- Use `-c sandbox_mode=danger-full-access -c approval_policy=never`.
- Switch to full access interactively via `/permissions` if the current config allows it.

Availability:

- **Interactive sessions**: yes, when started with one of the enabling flags or when allowed by config and selected via `/permissions`.
- **Non-interactive sessions**: yes, `codex exec --yolo` works, and is the intended way to run inside an externally hardened CI container.

When in YOLO mode:

- **Allowed**: almost all tool calls execute without prompting, including file edits anywhere on the filesystem, shell commands, network requests, MCP tool calls, subagent spawns, and web search.
- **Still constrained**: managed requirements can still disallow `danger-full-access` or `approval_policy = "never"`; if so, Codex falls back to a compliant value. Destructive app or MCP tool hints may still surface approval prompts when the tool advertises destructive side effects.
- **Not allowed**: it cannot override managed requirements that block full access.

### Root User

When Codex CLI is started as root or under `sudo` on macOS or Linux, the behavior depends on the sandbox mode and the platform:

- `read-only` and `workspace-write` modes work normally for root, subject to OS-level sandbox support.
- `danger-full-access` and `--yolo` are available to root; Codex does not refuse them based on uid alone. The documentation recommends running `--yolo` only inside externally hardened environments such as containers, not on a normal root shell.
- If the platform sandbox cannot be enforced (for example, inside a container that lacks `bwrap`/`seccomp` capabilities), Codex may refuse to run restricted modes and require `--sandbox danger-full-access` or `--yolo` so the outer container provides the isolation.

On native Windows, the `[windows].sandbox` setting (`elevated` or `unelevated`) affects sandbox strength, but root-like behavior is not a separate concept.

### Configuring the Default

Default permissions are configured through TOML files at several scopes:

- **User scope**: `~/.codex/config.toml` applies across all projects.
- **Repo/project scope**: `.codex/config.toml` applies to everyone working in the repository and can be checked into version control. Only loads for trusted projects.
- **Profile scope**: `~/.codex/profile-name.config.toml` applies when selected with `--profile profile-name`.
- **System scope**: `/etc/codex/config.toml` on Unix applies as a machine-wide default.
- **Managed scope**: cloud-managed or MDM-pushed `requirements.toml` and `/etc/codex/managed_config.toml` enforce defaults and constraints.

For the schema's `config_files` field, user scope is `~/.codex/config.toml` and repo scope is `.codex/config.toml`.

Examples that illustrate the grammar:

```toml
# ~/.codex/config.toml — user-wide legacy sandbox defaults
sandbox_mode = "workspace-write"
approval_policy = "on-request"

[sandbox_workspace_write]
network_access = false
```

```toml
# ~/.codex/config.toml — beta permission profile

default_permissions = "project-edit"

[permissions.project-edit]
extends = ":workspace"

[permissions.project-edit.filesystem.":workspace_roots"]
"**/*.env" = "deny"

[permissions.project-edit.network]
enabled = true

[permissions.project-edit.network.domains]
"api.openai.com" = "allow"
"*.github.com" = "allow"
```

```toml
# ~/.codex/readonly-quiet.config.toml — profile for CI
sandbox_mode = "read-only"
approval_policy = "never"
```

```toml
# ~/.codex/config.toml — granular approval policy
approval_policy = { granular = { sandbox_approval = true, rules = true, mcp_elicitations = true, request_permissions = false, skill_approval = false } }
```

### Extending the Base

Default permissions can be set at user scope and then narrowed or extended by narrower scopes.

**Example 1: user allows network, repo denies it.**

User `~/.codex/config.toml`:

```toml
[sandbox_workspace_write]
network_access = true
```

Repo `.codex/config.toml`:

```toml
[sandbox_workspace_write]
network_access = false
```

Result: network access is disabled in the repository because the project config overrides the user config.

**Example 2: user default mode, CLI override.**

User `~/.codex/config.toml`:

```toml
sandbox_mode = "workspace-write"
approval_policy = "on-request"
```

CLI:

```bash
codex --sandbox read-only --ask-for-approval untrusted
```

Result: the session starts in read-only sandbox with untrusted approval policy. CLI flags override settings.

**Example 3: repo permission profile, user profile layers on top.**

Repo `.codex/config.toml`:

```toml
default_permissions = "team-default"

[permissions.team-default]
extends = ":workspace"
```

CLI:

```bash
codex --profile stricter-network -c 'default_permissions = "stricter"'
```

Result: the CLI selects the `stricter` profile (defined in `~/.codex/stricter.config.toml`) and the `-c` override sets it as the active permission profile, taking precedence over the repo default.

## Tools and Permissions

Codex CLI provides a mix of built-in tools, app/connector tools, MCP tools, and subagent tools. The following table lists the common built-in tools and how permissions apply.

| Tool | Permission boundary |
| :--- | :--- |
| `shell` / `bash` | Governed by sandbox mode and approval policy. Read-only sandbox blocks writes; workspace-write allows edits inside the workspace. Network access requires `sandbox_workspace_write.network_access` or YOLO mode. |
| File read / `read_file` | Allowed inside the sandbox read surface. Outside the sandbox, workspace-write mode asks for approval. |
| File write / `apply_patch` | Allowed inside the sandbox write surface. Read-only mode blocks writes; workspace-write protects `.git`, `.codex`, and `.agents`. |
| `web_search` | Independent of command sandbox network. Defaults to cached mode; live mode requires `--search` or `web_search = "live"`. Disabled with `web_search = "disabled"`. |
| App/connector tools | Governed by `apps.<id>.enabled`, `default_tools_approval_mode`, per-tool `approval_mode`, `destructive_enabled`, and `open_world_enabled`. |
| MCP tools (`mcp__<server>__<tool>`) | Governed by MCP server config (`enabled_tools`, `disabled_tools`, `default_tools_approval_mode`, per-tool `approval_mode`) and the active approval policy. |
| `spawn_agent` / subagent tools | Subagents inherit the parent sandbox and approval policy at spawn time, including any live runtime overrides such as `/permissions` changes or `--yolo`. Custom agent files can override `sandbox_mode`, `default_permissions`, `approval_policy`, and `mcp_servers`. |
| `request_permissions` | Surface controlled by `approval_policy.granular.request_permissions`; can ask the user to escalate sandbox or approval mode. |
| `image_generation` | Gated by product availability and usage limits, not by sandbox mode. |

Permissions map to tool calls through the sandbox layer first, then the approval policy layer, then execpolicy `.rules` for command prefix decisions. A tool call is allowed only if the sandbox permits the underlying filesystem or network access, and then only if the approval policy does not require a prompt.

### Rule grammar details

Codex's `execpolicy` rules live in `.rules` files under `~/.codex/rules/` or `.codex/rules/`. They use Starlark syntax:

```starlark
prefix_rule(
    pattern = ["gh", "pr", "view"],
    decision = "prompt",
    justification = "Viewing PRs is allowed with approval",
)
```

- **`pattern`**: a non-empty list of literal strings or unions of literals defining the command prefix to match.
- **`decision`**: `allow`, `prompt`, or `forbidden`. Defaults to `allow`.
- **`justification`**: human-readable reason surfaced in prompts or rejections.
- **`match` / `not_match`**: optional inline examples validated at load time.

When multiple rules match, the most restrictive decision wins: `forbidden` > `prompt` > `allow`. Codex treats commands as argument lists. `bash -lc`, `bash -c`, and `zsh`/`sh` equivalents are parsed and split into separate commands when the script is a safe linear chain (`&&`, `||`, `;`, `|`). Advanced shell features disable splitting and the whole invocation is matched as a single command.

## Sandboxing, Trust, and Administrative Controls

### Sandboxing

Codex's sandbox is a separate layer from approval modes and rules. It provides OS-level filesystem and network isolation for spawned commands.

- **Backends**: macOS uses Seatbelt (`sandbox-exec`); Linux and WSL2 use `bubblewrap` with `seccomp` and a Landlock fallback; native Windows uses the Windows sandbox (`elevated` or `unelevated`).
- **Modes**:
  - `read-only`: inspect files only.
  - `workspace-write`: read, edit, and run routine commands inside the workspace boundary.
  - `danger-full-access`: no sandbox restrictions.
  - Beta permission profiles (`:read-only`, `:workspace`, `:danger-full-access` and custom profiles) provide reusable filesystem/network policies.
- **Filesystem**: by default, sandboxed commands can write only to the working directory and temp directories. Protected paths `.git`, `.codex`, and `.agents` are read-only inside writable roots; if `.git` is a pointer file, the resolved Git directory is also protected. Use `--add-dir` or profile `workspace_roots` to extend the writable surface. Use profile `filesystem` tables for fine-grained `read`/`write`/`deny` rules.
- **Network**: off by default. Enable with `sandbox_workspace_write.network_access = true` (legacy) or `permissions.<name>.network.enabled = true` (profiles). Constrain with `features.network_proxy` domain rules, Unix socket allowlists, and the local/private guard.
- **Failure behavior**: if the selected policy cannot be enforced by the platform sandbox, Codex refuses to run the command rather than silently running it unsandboxed.

Permissions and sandboxing are complementary:

- Permission rules and approval policy block Codex from attempting restricted actions.
- Sandbox restrictions prevent spawned commands from reaching resources outside defined boundaries, even if a prompt injection bypasses Codex's decision-making.

### Trust and administrative controls

**Folder/project trust**: first-time launches in a project directory prompt a workspace trust dialog. Untrusted folders ignore project `.codex/config.toml`, `.codex/rules/`, `.codex/agents/`, project hooks, project MCP servers, and project skills. Trust is saved per directory. In non-interactive `codex exec`, trust dialogs are skipped, so project-local surfaces load only if already trusted or allowed by user/managed settings.

**Managed/admin policy**: enterprise admins can deliver requirements and defaults:

- **Requirements** (`requirements.toml` via cloud, MDM `requirements_toml_base64`, or system `/etc/codex/requirements.toml`) enforce constraints. They can restrict allowed approval policies, sandbox modes, permission profiles, web search modes, MCP servers, plugin marketplace sources, hooks, and feature flags. They can also enforce `deny_read` paths and `experimental_network` rules.
- **Managed defaults** (`/etc/codex/managed_config.toml` or MDM `config_toml_base64`) provide starting values that override CLI `--config` and user config. Users can change them mid-session, but managed defaults are reapplied on next launch.

**Safe mode equivalent**: Codex has no single `--safe-mode` flag. Equivalent hardening can be achieved by disabling feature flags (`hooks`, `multi_agent`, `shell_tool`, `web_search`, `browser_use`, `computer_use`, `in_app_browser`) and using `--ignore-user-config` / `--ignore-rules`.

### Protected paths

In writable sandbox roots, the following paths remain read-only:

- `<writable_root>/.git`
- `<writable_root>/.codex`
- `<writable_root>/.agents`
- The resolved Git directory when `.git` is a pointer file

Managed `deny_read` requirements can add further protected paths.

## MCP and Permissions

MCP servers extend Codex with external tools. Their configuration lives in the same `config.toml` layers as other settings: user `~/.codex/config.toml`, project `.codex/config.toml` (trusted projects only), and custom agent files.

Permission controls for MCP:

- **Server enable/disable**: `mcp_servers.<id>.enabled = false` disables a server without removing its config; `required = true` fails startup if the server cannot initialize.
- **Tool allowlist/denylist**: `enabled_tools` and `disabled_tools` restrict which tools from a server are exposed. `disabled_tools` applies after `enabled_tools`.
- **Default approval mode**: `mcp_servers.<id>.default_tools_approval_mode` sets `auto`, `prompt`, or `approve` for all tools on that server unless overridden.
- **Per-tool approval mode**: `mcp_servers.<id>.tools.<tool>.approval_mode` overrides the default for a single tool.
- **Destructive hints**: tools that advertise `destructive_hint = true` always require approval when the active policy would otherwise auto-approve.
- **Managed requirements**: admins can restrict which MCP servers users may enable by defining approved identities in `requirements.toml` based on `command` (for stdio) or `url` (for HTTP). An empty `mcp_servers` table disables all MCP servers.
- **Plugin-bundled MCP servers**: installed plugins can bundle servers under `plugins.<plugin>.mcp_servers.<server>` with the same enable/tool/approval controls.

To make MCP safer:

- Use project-scoped `.codex/config.toml` only in trusted projects; untrusted project layers are skipped entirely.
- Set `default_tools_approval_mode = "prompt"` or `"approve"` only for servers you trust.
- Use `enabled_tools` to expose only the tools a workflow needs.
- Constrain command network access with `features.network_proxy` domain allowlists so MCP servers cannot reach arbitrary hosts even when command network access is on.
- In CI, use `codex exec --ignore-user-config --sandbox read-only --ask-for-approval never` or restrict MCP through managed requirements.

MCP tools run outside the Bash sandbox. Remote MCP servers make network requests from outside Codex's process, and stdio MCP servers run as local subprocesses with the environment limited by `env_vars`.

## Non-Interactive Behavior

In non-interactive `codex exec` mode, Codex cannot show interactive approval prompts. Use one of these strategies:

- Pass `--sandbox workspace-write` or `--sandbox danger-full-access` so actions stay inside the sandbox.
- Start with `--ask-for-approval never` and pre-define all needed permissions; actions that would have prompted fail and are reported back to the model.
- Use `--ignore-user-config` and `--ignore-rules` to make runs reproducible.
- Use `--json` to consume events programmatically.

If a configured MCP server has `required = true` and fails to initialize, `codex exec` exits with an error instead of continuing without that server.

## Sources

- [Codex CLI overview](https://developers.openai.com/codex/cli)
- [Command line options](https://developers.openai.com/codex/cli/reference)
- [Config basics](https://developers.openai.com/codex/config-basic)
- [Configuration reference](https://developers.openai.com/codex/config-reference)
- [Permissions](https://developers.openai.com/codex/permissions)
- [Agent approvals & security](https://developers.openai.com/codex/agent-approvals-security)
- [Sandboxing](https://developers.openai.com/codex/concepts/sandboxing)
- [Rules](https://developers.openai.com/codex/rules)
- [MCP](https://developers.openai.com/codex/mcp)
- [Non-interactive mode](https://developers.openai.com/codex/noninteractive)
- [Subagents](https://developers.openai.com/codex/subagents)
- [Managed configuration](https://developers.openai.com/codex/enterprise/managed-configuration)
- [Environment variables](https://developers.openai.com/codex/environment-variables)
- [OpenAI Codex repository](https://github.com/openai/codex)

## Changelog

- 2026-07-02: Refreshed research against current Codex CLI documentation and local `~/.codex/config.toml`. Added beta permission profiles, execpolicy rules, managed requirements/defaults, granular approval policy, Auto-review, custom agent overrides, expanded sandbox backend details, MCP server identity filtering, and updated protected paths. Updated frontmatter to the full schema contract and flagged Claudine updates as required.
