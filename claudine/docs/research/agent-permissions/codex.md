---
$schema: ./_schema.yaml
created: 2026-07-01
last_updated: 2026-07-03
agent: codex
model: default

cli_params:
  - param: sandbox
    style: switch
    description: "Selects the sandbox policy for model-generated shell commands. Values: read-only, workspace-write, danger-full-access."
    example: "codex --sandbox workspace-write"
    example_description: "Starts an interactive session that can edit the workspace but still prompts for work outside the sandbox."
  - param: ask-for-approval
    style: switch
    description: "Controls when Codex pauses for human approval before running commands. Values: untrusted, on-request, never; on-failure is deprecated."
    example: "codex --ask-for-approval on-request"
    example_description: "Lets Codex run sandbox-safe actions and ask for escalations."
  - param: dangerously-bypass-approvals-and-sandbox
    style: switch
    description: "Disables approval prompts and sandboxing for the session. Alias: --yolo."
    example: "codex --dangerously-bypass-approvals-and-sandbox"
    example_description: "Runs in full-access mode and should only be used inside an externally sandboxed environment."
  - param: add-dir
    style: switch
    description: "Adds writable workspace roots alongside the primary working directory; repeatable."
    example: "codex --add-dir ../shared --add-dir ../docs"
    example_description: "Lets Codex write to selected sibling directories for this session."
  - param: config
    style: switch
    description: "Overrides a configuration key for this invocation. Values are parsed as TOML when possible and support dotted keys."
    example: "codex -c 'approval_policy=\"never\"' -c 'sandbox_mode=\"read-only\"'"
    example_description: "Applies a session-scoped locked-down approval and sandbox posture without editing config files."
  - param: profile
    style: switch
    description: "Layers $CODEX_HOME/<name>.config.toml on top of base user config."
    example: "codex --profile readonly-ci"
    example_description: "Loads a saved profile that can set approval, sandbox, MCP, and feature defaults."
  - param: ignore-user-config
    style: switch
    description: "codex exec only. Skips loading $CODEX_HOME/config.toml while still using CODEX_HOME for auth and state."
    example: "codex exec --ignore-user-config --sandbox read-only 'summarize the repo'"
    example_description: "Runs automation without user-level permission defaults."
  - param: ignore-rules
    style: switch
    description: "codex exec only. Skips user and project execpolicy .rules files."
    example: "codex exec --ignore-rules --sandbox read-only 'inspect the tree'"
    example_description: "Avoids persistent command allowlists during a controlled run."
  - param: skip-git-repo-check
    style: switch
    description: "codex exec only. Allows running outside a Git repository; relevant because default posture differs for version-controlled folders."
    example: "codex exec --skip-git-repo-check --sandbox read-only 'summarize these files'"
    example_description: "Runs a headless task in a non-repository directory."
  - param: search
    style: switch
    description: "Enables live web search by setting web_search to live; without it Codex uses cached search unless full-access mode makes live search the default."
    example: "codex --search 'check the latest upstream API notes'"
    example_description: "Adds live web search to the model-visible tool surface."
  - param: enable
    style: switch
    description: "Force-enables a feature flag for this invocation; equivalent to -c features.<name>=true."
    example: "codex --enable network_proxy"
    example_description: "Turns on an experimental feature for the session."
  - param: disable
    style: switch
    description: "Force-disables a feature flag for this invocation; equivalent to -c features.<name>=false."
    example: "codex --disable shell_tool"
    example_description: "Removes a feature-gated tool category from the session when that feature is available."
  - param: dangerously-bypass-hook-trust
    style: switch
    description: "Runs enabled hooks without requiring persisted hook trust for this invocation."
    example: "codex --dangerously-bypass-hook-trust"
    example_description: "Allows automation that vets hooks elsewhere to bypass Codex's hook trust gate."
  - param: cd
    style: switch
    description: "Sets the agent working root before the session starts; affects workspace boundary, project config discovery, and project trust lookup."
    example: "codex --cd ./packages/api --sandbox workspace-write"
    example_description: "Runs Codex with a narrower project root."
  - param: strict-config
    style: switch
    description: "Errors when config.toml contains fields unknown to this Codex version."
    example: "codex --strict-config --sandbox read-only"
    example_description: "Fails closed on stale or misspelled config keys."
  - param: permissions-profile
    style: switch
    description: "codex sandbox subcommand only. Selects a named permission profile from the active configuration stack."
    example: "codex sandbox --permissions-profile project-edit -- cargo check"
    example_description: "Runs one command under a named beta filesystem/network profile."
  - param: include-managed-config
    style: switch
    description: "codex sandbox subcommand only. Includes managed requirements while resolving an explicit permissions profile."
    example: "codex sandbox --include-managed-config --permissions-profile project-edit -- npm test"
    example_description: "Tests the command sandbox against enterprise constraints."
  - param: allow-unix-socket
    style: switch
    description: "codex sandbox subcommand only. Allows the sandboxed command to bind or connect AF_UNIX sockets rooted at the given path; repeatable."
    example: "codex sandbox --allow-unix-socket ./run -- ./script.sh"
    example_description: "Adds a narrow Unix-socket exception for a sandboxed command."

env_vars:
  - name: CODEX_HOME
    effect: "Sets the root for Codex state, including config.toml, profile files, auth, logs, sessions, skills, rules, agents, plugins, and package metadata. This changes which permission config and rules are loaded."
    effect_category: state_home_relocation
  - name: CODEX_SQLITE_HOME
    effect: "Sets where SQLite-backed state is stored; sqlite_home config takes precedence. It does not directly set permissions but can affect persisted trust/session state location."
    effect_category: state_home_relocation

config_files:
  - os: macos
    user: ".codex/config.toml"
    repo: ".codex/config.toml"
    notes: "User path is relative to $HOME unless CODEX_HOME points elsewhere. User rules live under .codex/rules/, profiles under .codex/<name>.config.toml, user agents under .codex/agents/. System config and managed defaults can live under /etc/codex/; macOS MDM can provide managed requirements/defaults via com.openai.codex."
  - os: linux
    user: ".codex/config.toml"
    repo: ".codex/config.toml"
    notes: "User path is relative to $HOME unless CODEX_HOME points elsewhere. User rules live under .codex/rules/, profiles under .codex/<name>.config.toml, user agents under .codex/agents/. System config and managed defaults can live under /etc/codex/."
  - os: windows
    user: ".codex/config.toml"
    repo: ".codex/config.toml"
    notes: "User path is relative to the user's home directory unless CODEX_HOME points elsewhere. Managed requirements live under %ProgramData%\\OpenAI\\Codex\\requirements.toml. Native Windows sandbox behavior is controlled by windows.sandbox and related requirements."

precedence:
  - source: managed_requirements
    scope: ["approval_mode", "sandbox", "rules", "mcp", "other", "hooks", "extensions"]
    merge_strategy: shallow
    notes: "Cloud-managed requirements, macOS MDM requirements, and system requirements.toml constrain lower sources. First requirement source wins for a setting; some tables combine entry-by-entry."
  - source: managed_defaults
    scope: ["approval_mode", "sandbox", "rules", "mcp", "other", "hooks"]
    merge_strategy: none
    notes: "Managed defaults apply at launch and are reapplied on next start; users can still change settings during a session unless a requirement constrains them."
  - source: cli
    scope: ["approval_mode", "sandbox", "rules", "tool_visibility", "config_loading", "workspace"]
    merge_strategy: none
    notes: "CLI flags and -c overrides apply to one invocation and override ordinary config. They cannot override managed requirements."
  - source: project_config
    scope: ["approval_mode", "sandbox", "rules", "mcp", "agents", "hooks", "skills"]
    merge_strategy: nearest
    notes: "Project .codex/config.toml and .codex/rules/ load only for trusted projects. Project config cannot override machine-local provider/auth/profile/telemetry/notification keys."
  - source: profile
    scope: ["approval_mode", "sandbox", "rules", "mcp", "other"]
    merge_strategy: shallow
    notes: "$CODEX_HOME/<name>.config.toml is selected with --profile and layers on top of base user config."
  - source: user_config
    scope: ["approval_mode", "sandbox", "rules", "mcp", "agents", "hooks", "skills"]
    merge_strategy: shallow
    notes: "$CODEX_HOME/config.toml, $CODEX_HOME/rules/, and $CODEX_HOME/agents/ define the user baseline. In this session CODEX_HOME resolved to /Users/ken/.claudine/.codex and no TOML/rules config files were present there."
  - source: system_config
    scope: ["general_config"]
    merge_strategy: none
    notes: "Unix system config can provide lower-precedence machine defaults."
  - source: built_in_defaults
    scope: ["approval_mode", "sandbox", "tool_visibility", "mcp", "rules"]
    merge_strategy: none
    notes: "Defaults depend on surface and trust: interactive Codex recommends Auto for version-controlled folders and read-only for non-version-controlled or untrusted folders; codex exec defaults to read-only."

default_posture: "Interactive Codex detects the working directory and recommends Auto (workspace-write plus on-request approvals) for version-controlled trusted folders, but may start read-only until the folder is trusted; non-version-controlled folders are read-only by default. codex exec defaults to a read-only sandbox unless flags or config grant more."

cli_zero_permissions:
  supported: true
  invocation: "codex exec --ignore-user-config --ignore-rules --sandbox read-only --ask-for-approval never"
  mechanism: "Read-only sandbox plus never-approve headless policy; ignores user config and execpolicy allowlists for the run."
  limitations: "Codex still has read access inside the sandbox read surface and still exposes non-shell model capabilities needed for the session. There is no single CLI flag to hide every built-in tool; add-back is via --sandbox, --add-dir, --ask-for-approval, --search, feature flags, or -c overrides."

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
  mechanism: "--dangerously-bypass-approvals-and-sandbox / --yolo, or the equivalent combination of --sandbox danger-full-access and --ask-for-approval never; interactive sessions can also change permissions with /permissions when not blocked by managed requirements."

policy_engine:
  ergonomic: false
  provides_coverage: false
  gaps:
    - "Codex separates OS sandbox/profile enforcement from approval prompts; PolicyEngine models a flatter allow/ask/deny policy."
    - "Beta permission profiles have path tokens, scoped subpaths, glob deny rules, network domain controls, and profile inheritance that PolicyEngine cannot express completely."
    - "Execpolicy rules are Starlark prefix rules with command-list matching and shell-splitting semantics."
    - "Managed requirements constrain lower sources instead of simply replacing them."
    - "Approval policy can be granular by category: sandbox approvals, rules, MCP elicitations, request_permissions, and skill approvals."
    - "Tool visibility is spread across feature flags, web_search mode, MCP/app enabled_tools and disabled_tools, and plugins rather than one provider-native tool list."
    - "Custom agents can override normal session config while also inheriting live parent runtime overrides."

permission_entities:
  - entity: tool
    native_names: ["shell", "apply_patch", "web_search", "image_generation", "request_permissions"]
    notes: "Built-in tool calls are controlled by sandbox/profile, approval policy, and feature/web_search settings."
  - entity: tool_group
    native_names: ["features.<name>", "web_search"]
    notes: "Feature flags and web_search mode can remove broad tool categories from model visibility."
  - entity: command
    native_names: ["prefix_rule", "rules.prefix_rules", "approval_policy"]
    notes: "Command execution is evaluated against the sandbox first, then approval/rule behavior for escalations."
  - entity: path
    native_names: ["sandbox_mode", "sandbox_workspace_write", "permissions.<name>.filesystem", "deny_read", "--add-dir"]
    notes: "Paths can be controlled coarsely by sandbox mode or narrowly by permission-profile filesystem entries."
  - entity: workspace
    native_names: ["workspace roots", "permissions.<name>.workspace_roots", ":workspace_roots", "--cd", "--add-dir"]
    notes: "Workspace roots define the default write/read boundary and protected metadata subpaths."
  - entity: mcp_server
    native_names: ["mcp_servers.<name>", "mcp_servers.<name>.enabled", "mcp_servers.<name>.required", "managed mcp_servers"]
    notes: "Server availability can be configured locally and constrained by managed identity allowlists."
  - entity: mcp_tool
    native_names: ["enabled_tools", "disabled_tools", "default_tools_approval_mode", "tools.<tool>.approval_mode"]
    notes: "Tool filters hide/allow MCP tools and set per-tool approval behavior."
  - entity: mcp_resource
    native_names: ["MCP resources"]
    notes: "Codex documents MCP tool and server controls; no separate resource-specific approval grammar was found."
  - entity: agent
    native_names: ["agents.<name>", "spawn_agent", "spawn_agents_on_csv"]
    notes: "Subagent workflows are enabled by default and have concurrency/depth limits."
  - entity: subagent
    native_names: ["~/.codex/agents/*.toml", ".codex/agents/*.toml"]
    notes: "Custom agent TOML files are config layers and can override sandbox, permissions, model, MCP, and skills settings."
  - entity: mode
    native_names: ["Auto", "Read-only", "Full Access", "sandbox_mode", "default_permissions"]
    notes: "Interactive mode names map onto sandbox and approval settings."
  - entity: approval_category
    native_names: ["approval_policy", "approval_policy.granular", "approvals_reviewer"]
    notes: "Granular policy controls prompt categories and auto-review can review eligible approval requests."
  - entity: sandbox
    native_names: ["read-only", "workspace-write", "danger-full-access", "permission profiles", "windows.sandbox"]
    notes: "The sandbox is the OS-enforced boundary for spawned commands."
  - entity: hook
    native_names: ["hooks", "allow_managed_hooks_only", "--dangerously-bypass-hook-trust"]
    notes: "Hook trust and managed-hooks-only policy are separate administrative controls."
  - entity: extension
    native_names: ["plugins", "marketplaces", "plugins.<name>.mcp_servers"]
    notes: "Plugins can contribute MCP servers/tools and marketplace sources can be constrained by managed policy."
  - entity: slash_command
    native_names: ["/permissions", "/status", "/mcp", "/agent", "/approve"]
    notes: "Slash commands expose runtime security state and, in interactive sessions, permission changes or approval retries."

approval_modes:
  - name: untrusted
    effect: "Only trusted/read-oriented commands run automatically; other command requests ask for approval."
    interactive: true
    non_interactive: true
    aliases: ["--ask-for-approval untrusted", "-a untrusted", "approval_policy = \"untrusted\""]
  - name: on-request
    effect: "Sandbox-safe actions run automatically; model-requested escalations, network, and out-of-sandbox work prompt."
    interactive: true
    non_interactive: true
    aliases: ["--ask-for-approval on-request", "-a on-request", "approval_policy = \"on-request\"", "Auto"]
  - name: never
    effect: "Never prompt; actions that need approval fail or are rejected."
    interactive: true
    non_interactive: true
    aliases: ["--ask-for-approval never", "-a never", "approval_policy = \"never\""]
  - name: granular
    effect: "Per-category approval toggles for sandbox approvals, execpolicy rules, MCP prompts, request_permissions prompts, and skill approvals."
    interactive: true
    non_interactive: true
    aliases: ["approval_policy = { granular = { ... } }"]
  - name: Full Access
    effect: "No sandbox restrictions and no approval prompts unless constrained by managed policy or separate tool/app semantics."
    interactive: true
    non_interactive: true
    aliases: ["--yolo", "--dangerously-bypass-approvals-and-sandbox", "--sandbox danger-full-access --ask-for-approval never", ":danger-full-access"]
  - name: Read-only
    effect: "Commands can inspect permitted files but cannot write; interactive users can approve plans or switch mode."
    interactive: true
    non_interactive: true
    aliases: ["--sandbox read-only", ":read-only"]
  - name: auto_review
    effect: "Routes eligible approval requests through a reviewer agent before execution."
    interactive: true
    non_interactive: false
    aliases: ["approvals_reviewer = \"auto_review\""]

rule_model:
  decisions: ["allow", "prompt", "forbidden", "read", "write", "deny"]
  syntax: "Execpolicy .rules files use side-effect-free Starlark prefix_rule(pattern=[...], decision='allow'|'prompt'|'forbidden', justification='...', match=[...], not_match=[...]). Permission profiles use TOML filesystem entries with read/write/deny and network entries with allow/deny-style domain policy."
  precedence: "Execpolicy conflicts choose the most restrictive decision: forbidden > prompt > allow. Permission-profile filesystem conflicts choose the most specific path; exact ties use deny > write > read."
  merge_semantics: "Rules load from every active config layer and managed requirements can add restrictive rules. Project rules/config require trust. Managed requirements constrain lower sources; project/user/profile/CLI config otherwise layer by normal config precedence."
  matcher_semantics: "Execpolicy matches exact argv prefixes with literal tokens or any_of-style alternatives. Safe linear bash/sh/zsh scripts can be split and each command evaluated; advanced shell syntax is treated as the whole shell invocation. Permission profile paths support special tokens, absolute paths, home-relative paths, workspace-relative subpaths, and glob deny/read/write entries."
  default_decision: "If no execpolicy rule matches, Codex falls back to sandbox/profile and approval policy. If no permission profile is selected, legacy sandbox_mode or built-in defaults apply."

tool_visibility:
  supported: true
  mechanisms:
    - "web_search = \"disabled\" hides web search; --search or web_search = \"live\" enables live search."
    - "--enable/--disable feature flags can add or remove feature-gated tool groups."
    - "MCP/app enabled_tools and disabled_tools filter server tools, with disabled_tools applied after enabled_tools."
    - "Managed requirements can constrain feature flags, marketplace/plugin sources, MCP server identities, and permission profiles."
  notes: "Codex does not expose a single --tools allowlist for built-in tools in the observed CLI. Tool visibility and tool approval are separate control surfaces."

sandbox:
  supported: true
  modes: ["read-only", "workspace-write", "danger-full-access", ":read-only", ":workspace", ":danger-full-access", "custom [permissions.<name>] profiles"]
  backends: ["macOS Seatbelt / sandbox-exec", "Linux and WSL bubblewrap/seccomp with Landlock-related fallback behavior", "native Windows sandbox with elevated or unelevated mode"]
  filesystem_control: "Legacy sandbox modes provide coarse read/write roots; beta permission profiles provide read/write/deny path rules with special tokens, workspace roots, exact paths, home-relative paths, and globs. workspace-write protects .git, .agents, and .codex under writable roots."
  network_control: "Network is off by default for command sandboxes. Legacy workspace-write network can be enabled with sandbox_workspace_write.network_access; permission profiles have network.enabled and domain controls; feature/network proxy settings can narrow network destinations. Web search is a separate model tool surface."
  notes: "Sandboxing applies to spawned local commands. MCP servers and remote app tools are separate processes/services and should not be assumed to run inside the command sandbox."

trust_and_admin:
  folder_trust: "Project .codex/config.toml, .codex/rules/, .codex/agents/, project MCP servers, hooks, and skills require project trust. Untrusted or non-version-controlled folders can start read-only."
  managed_policy: "ChatGPT Business/Enterprise cloud-managed requirements, macOS MDM requirements, system requirements.toml, and managed defaults can constrain approval policy, sandbox modes, permission profiles, rules, MCP servers, hooks, feature flags, and marketplaces."
  safe_mode: "No single safe-mode flag was found. A locked-down run combines --ignore-user-config, --ignore-rules, read-only sandbox, approval_policy never/untrusted, disabled feature flags, and no project trust."
  notes: "Managed requirements constrain lower sources; managed defaults are launch defaults reapplied on next start. allow_managed_hooks_only in requirements.toml ignores user, project, and session hook config while keeping managed hooks."

mcp_permissions:
  supported: true
  server_filters:
    - "mcp_servers.<server>.enabled"
    - "mcp_servers.<server>.required"
    - "managed requirements mcp_servers identity allowlists"
    - "project-scoped MCP servers gated by project trust"
  tool_filters:
    - "mcp_servers.<server>.enabled_tools"
    - "mcp_servers.<server>.disabled_tools"
    - "mcp_servers.<server>.default_tools_approval_mode"
    - "mcp_servers.<server>.tools.<tool>.approval_mode"
  trust_model: "MCP configuration can live in user or trusted project config. STDIO servers run as local processes with configured env/env_vars/cwd; HTTP servers are remote services. OAuth credentials are user state."
  notes: "MCP approval modes are auto, prompt, and approve. disabled_tools is applied after enabled_tools. No documented MCP response-sanitization/interception layer was found; resource access is effectively server/tool scoped."

headless_behavior: "codex exec has no interactive TUI prompt surface. It defaults to read-only; actions that need new approval fail or are reported back to the model unless the run is preconfigured with enough sandbox/approval permissions. Required MCP startup failures make codex exec exit with an error."

approval_persistence: "Interactive allow-list additions are written to $CODEX_HOME/rules/default.rules and persist across sessions. Runtime /permissions changes are session-scoped; managed defaults are reapplied on the next launch."

protected_paths:
  - "<writable_root>/.git"
  - "resolved Git directory from a <writable_root>/.git pointer file"
  - "<writable_root>/.agents"
  - "<writable_root>/.codex"

security_posture: "Codex combines an OS-enforced sandbox for local spawned commands, advisory/interactive approval UX, static Starlark command rules, beta filesystem/network permission profiles, and managed enterprise constraints. The sandbox is the strongest boundary; approvals, rules, MCP filters, hooks, and feature flags are policy/control-plane layers rather than a universal OS sandbox."

changes:
  - "Set schema to ./_schema.yaml and converted config_files from the prior invalid os: all shape to separate macOS, Linux, and Windows records."
  - "Corrected the agent/model metadata to agent: codex and model: default."
  - "Verified installed Codex CLI version 0.142.5 and refreshed CLI params from codex --help, codex exec --help, codex sandbox --help, codex mcp --help, codex plugin --help, and codex features --help."
  - "Recorded the observed local CODEX_HOME for this session as /Users/ken/.claudine/.codex and noted that no local TOML/rules files existed there to inspect."
  - "Updated the default posture to distinguish interactive launch recommendations from codex exec's documented read-only default."
  - "Added codex sandbox permission-profile controls and Unix-socket exceptions to the adjacent security-control CLI surface."
  - "Verified current docs for permission profiles, protected paths, execpolicy rules, MCP filters, subagent inheritance, non-interactive behavior, and managed requirements."
  - "Kept the prior conclusion that Claudine needs PolicyEngine extensions for accurate Codex coverage."

requires_claudine_update: true
reason: "Codex requires Claudine to model separate sandbox/profile enforcement, approval policy, execpolicy Starlark rules, managed constraints, tool visibility, and custom agent inheritance. The current PolicyEngine cannot fully represent those axes or their precedence."
---

# Codex CLI Permissions and Security Controls

## Introduction to Codex CLI Permissions

Codex CLI has two core permission layers:

- **Sandbox or permission profile**: what local model-generated commands can technically access.
- **Approval policy**: when Codex asks before taking an action.

Those layers are related but not identical. A command can be blocked by the sandbox without an approval prompt, and a visible tool can still need approval. Tool visibility is a third concern: Codex can hide or disable tool surfaces with feature flags, `web_search = "disabled"`, and MCP/app tool filters even when the approval policy would otherwise allow a visible tool.

Configuration lives primarily in `$CODEX_HOME/config.toml`, normally `~/.codex/config.toml`. Trusted projects can add `.codex/config.toml`, `.codex/rules/`, and `.codex/agents/`. Profile files live beside user config as `$CODEX_HOME/<name>.config.toml` and are selected with `--profile`.

In this local session, `$HOME` is `/Users/ken/.claudine`, so the active default `CODEX_HOME` is `/Users/ken/.claudine/.codex`. That directory exists, but no TOML config files or `.rules` files were present to inspect; only SQLite/state and prompt directories were found.

The relevant environment variables are sparse. `CODEX_HOME` is the important one because it changes where config, rules, agents, and persisted trust/state are read from. `CODEX_SQLITE_HOME` can move SQLite state, which may affect persisted session/trust state location, but it does not grant permissions by itself.

The main CLI switches are in the frontmatter. Highest-impact switches are `--sandbox`, `--ask-for-approval`, `--dangerously-bypass-approvals-and-sandbox`/`--yolo`, `--add-dir`, `-c/--config`, `--profile`, `--ignore-user-config`, `--ignore-rules`, `--search`, `--enable`, `--disable`, and `--dangerously-bypass-hook-trust`. `codex sandbox` also has `--permissions-profile`, `--include-managed-config`, and `--allow-unix-socket` for testing or running a single command under a resolved profile.

Precedence is not a simple total order because managed requirements constrain lower layers. Practically:

- Managed requirements win as constraints.
- Managed defaults provide launch-time starting values.
- CLI flags and `-c` overrides win over ordinary file config for one invocation.
- Trusted project config/rules override or extend user config in the project.
- Profile files layer on top of base user config when selected.
- Built-in defaults fill whatever remains.

## Permissions Use Cases

### Default

For interactive `codex`, the documented launch recommendation is `Auto` for version-controlled folders: workspace-write sandbox plus `on-request` approvals. Non-version-controlled folders start read-only, and Codex may also start read-only until the working directory is trusted. `codex exec` is different: it defaults to a read-only sandbox for automation.

PolicyEngine can approximate this by allowing reads in workspace/temp roots, allowing writes in workspace roots except protected metadata paths, denying or asking for outside writes, and denying network by default. It is not ergonomic because Codex derives behavior from launch surface, VCS/trust state, sandbox/profile choice, and approval policy rather than one rule table. Without changes, PolicyEngine cannot fully express Codex's dynamic default, protected metadata paths, or separation between OS sandbox and approval policy.

### Whitelisting

For a locked-down, session-scoped start without mutating provider config:

```bash
codex exec --ignore-user-config --ignore-rules --sandbox read-only --ask-for-approval never "summarize the repo"
```

This is the best CLI-only baseline for Claudine wrapper use. It skips user config and persistent execpolicy allowlists, uses the read-only sandbox, and rejects approval-required work because headless runs cannot prompt.

Interactive exploration can use:

```bash
codex --sandbox read-only --ask-for-approval untrusted "explain this repository"
```

Add back specific permissions with CLI flags:

```bash
codex exec --sandbox workspace-write "update docs"
codex exec --sandbox workspace-write --add-dir ../shared "update both packages"
codex --search "check current upstream release notes"
codex -c 'sandbox_workspace_write.network_access=true' --sandbox workspace-write "run integration checks"
codex -c 'default_permissions="project-edit"' "work under a named permission profile"
```

PolicyEngine can describe the intent as deny-by-default with explicit read/write/execute/network grants. The mismatch is that Codex has no true no-read or no-tools baseline for the normal CLI; read-only still permits reads inside the sandbox, and built-in tool visibility is not controlled by one allowlist. Current PolicyEngine coverage is therefore partial.

### YOLO

YOLO mode is available in interactive and non-interactive sessions through:

- `--dangerously-bypass-approvals-and-sandbox`
- `--yolo`
- `--sandbox danger-full-access --ask-for-approval never`
- equivalent `-c` overrides when not blocked by managed requirements
- interactive `/permissions` changes when permitted

YOLO allows local commands to run without Codex's sandbox and without approval prompts. It is intended only when an outer environment, such as a container or isolated CI runner, is the real security boundary. It does not override managed requirements that disallow full access or `approval_policy = "never"`, and separate app/MCP/destructive-tool semantics may still matter.

### Root User

No current documentation or observed help indicates that Codex refuses YOLO solely because the process runs as root. Running as root makes `danger-full-access` more dangerous because the outer operating-system account has more authority. Restricted sandbox modes still depend on platform sandbox availability; if the sandbox backend cannot enforce the requested policy, Codex should fail rather than silently run unsandboxed.

### Configuring the Default

User scope:

```toml
# ~/.codex/config.toml or $CODEX_HOME/config.toml
sandbox_mode = "workspace-write"
approval_policy = "on-request"

[sandbox_workspace_write]
network_access = false
```

Repo scope:

```toml
# .codex/config.toml, loaded only when the project is trusted
approval_policy = "on-request"
default_permissions = "project-edit"

[permissions.project-edit]
extends = ":workspace"

[permissions.project-edit.filesystem.":workspace_roots"]
"." = "write"
"**/*.env" = "deny"

[permissions.project-edit.network]
enabled = false
```

Rules:

```starlark
# ~/.codex/rules/default.rules or .codex/rules/team.rules
prefix_rule(
    pattern = ["gh", "pr", "view"],
    decision = "prompt",
    justification = "Viewing pull requests can disclose external data",
)
```

MCP tool filtering:

```toml
[mcp_servers.docs]
command = "npx"
args = ["-y", "@example/docs-mcp"]
enabled_tools = ["search_docs"]
disabled_tools = ["write_docs"]
default_tools_approval_mode = "prompt"

[mcp_servers.docs.tools.search_docs]
approval_mode = "auto"
```

### Extending the Base

User config can establish a broad default, and a trusted project can narrow it:

```toml
# user
sandbox_mode = "workspace-write"
approval_policy = "on-request"

[sandbox_workspace_write]
network_access = true
```

```toml
# repo
[sandbox_workspace_write]
network_access = false
```

CLI flags can narrow both:

```bash
codex --sandbox read-only --ask-for-approval untrusted
```

Managed requirements can constrain every lower source:

```toml
allowed_approval_policies = ["untrusted", "on-request"]
allowed_sandbox_modes = ["read-only", "workspace-write"]
```

That example blocks `--yolo` because `approval_policy = "never"` and `danger-full-access` are not allowed.

## Tools and Permissions

Default or common Codex tool surfaces include local shell/command execution, file reads, file edits/apply patch, web search, MCP tools, app/connector tools, image generation when available, `request_permissions`, and subagent tools such as `spawn_agent`.

| Tool surface | Permission boundary |
| --- | --- |
| Shell / local command | Sandbox/profile plus approval policy plus execpolicy rules. |
| File read | Sandbox/profile read surface; permission profiles can deny sensitive globs. |
| File write / apply patch | Sandbox/profile write surface; protected metadata paths remain guarded in workspace-write. |
| Web search | `web_search` config and `--search`; separate from command sandbox network. |
| MCP tools | Server enabled/required state, tool filters, approval modes, and managed MCP allowlists. |
| Subagents | Inherit parent runtime sandbox/approval overrides; custom agent TOML can set defaults. |
| Hooks | Hook trust, `--dangerously-bypass-hook-trust`, and managed-hooks-only policy. |

Native permission entities are listed in frontmatter. The key modeling point for Claudine is that Codex permissions target commands, paths, workspaces, MCP servers/tools, agents, modes, approval categories, sandboxes, hooks, plugins, and slash-command-driven runtime state. Treating this as only "tools allowed" loses important behavior.

Codex's rule grammar has two distinct pieces:

- `.rules` files use Starlark `prefix_rule()` with `allow`, `prompt`, and `forbidden`; conflicts choose `forbidden > prompt > allow`.
- Permission profiles use TOML filesystem and network rules. Filesystem entries use `read`, `write`, and `deny`; more specific paths win, and exact conflicts choose `deny > write > read`.

Approvals can persist when interactive allow-list changes are written to `$CODEX_HOME/rules/default.rules`. Session mode changes made through `/permissions` are runtime/session state, not permanent config edits.

## Sandboxing, Trust, and Administrative Controls

Sandboxing is separate from approval mode. Codex uses OS mechanisms for spawned commands: Seatbelt on macOS, bubblewrap/seccomp with Landlock-related behavior on Linux/WSL, and native Windows sandbox modes on Windows. The sandbox controls filesystem access and command network access. Web search, MCP servers, remote app tools, hooks, and plugins are separate surfaces and need separate policy modeling.

`workspace-write` allows workspace edits but protects `.git`, `.agents`, and `.codex` under writable roots. If `.git` is a pointer file, the resolved Git directory is also read-only. Permission profiles can use path tokens such as `:minimal`, `:workspace_roots`, `:tmpdir`, `:slash_tmp`, `:root`, absolute paths, home-relative paths, scoped workspace subpaths, and globs.

Folder/project trust gates project-local `.codex/config.toml`, `.codex/rules/`, `.codex/agents/`, project MCP servers, hooks, and skills. Project config cannot override machine-local provider/auth/profile/telemetry/notification keys.

Managed requirements can constrain approval policies, sandbox modes, permission profiles, web search, hooks, feature flags, MCP servers, and marketplaces. Managed defaults are launch-time starting values that can be changed during a session unless constrained, then reapplied next launch.

Security posture: Codex is a combination of OS-enforced sandbox for local commands, static policy rules/profiles, managed policy, and advisory approval UX. The OS sandbox does not automatically cover every provider surface.

## MCP and Permissions

MCP configuration can live in user config or trusted project config. Codex supports STDIO and streamable HTTP MCP servers. STDIO servers are local processes with configured command, args, env, env_vars, and cwd. HTTP servers are remote services with bearer/OAuth/header configuration.

MCP safety controls:

- Disable a server with `enabled = false`.
- Require a server with `required = true`, causing startup failure if it cannot initialize.
- Use `enabled_tools` as an allowlist and `disabled_tools` as a denylist; the denylist is applied after the allowlist.
- Set `default_tools_approval_mode` or per-tool `tools.<tool>.approval_mode` to `auto`, `prompt`, or `approve`.
- Use managed requirements to allow only approved MCP server identities.
- Keep project MCP config behind project trust.
- Limit STDIO environment exposure with `env_vars` instead of inheriting arbitrary secrets.

No current documentation shows MCP response interception/sanitization or resource-specific policy separate from server/tool availability. MCP servers should be modeled as outside the local command sandbox unless a specific server is itself launched through a sandboxed mechanism.

## Non-Interactive Behavior

`codex exec` runs without the interactive TUI. It streams progress to stderr and final output to stdout, and it defaults to read-only. Approval-required actions cannot ask the user in the normal overlay; they fail or are reported back to the model unless the run was preconfigured with enough permission. Required MCP server startup failure exits the run with an error.

For automation, use explicit flags. New scripts should prefer `--sandbox workspace-write` or `--sandbox danger-full-access` over deprecated compatibility shortcuts such as `--full-auto`, and use `--ignore-user-config` / `--ignore-rules` when user config would make automation nondeterministic.

## Sources

- [Codex CLI command line options](https://developers.openai.com/codex/cli/reference)
- [Agent approvals and security](https://developers.openai.com/codex/agent-approvals-security)
- [Codex permissions](https://developers.openai.com/codex/permissions)
- [Codex configuration reference](https://developers.openai.com/codex/config-reference)
- [Codex environment variables](https://developers.openai.com/codex/environment-variables)
- [Codex rules](https://developers.openai.com/codex/rules)
- [Codex MCP](https://developers.openai.com/codex/mcp)
- [Codex subagents](https://developers.openai.com/codex/subagents)
- [Codex non-interactive mode](https://developers.openai.com/codex/noninteractive)
- [Codex managed configuration](https://developers.openai.com/codex/enterprise/managed-configuration)
- [OpenAI Codex repository](https://github.com/openai/codex)
- Local installed CLI help from `codex-cli 0.142.5`: `codex --help`, `codex exec --help`, `codex sandbox --help`, `codex mcp --help`, `codex plugin --help`, and `codex features --help`.

## Changelog

- 2026-07-03: Refreshed against Codex CLI 0.142.5, current OpenAI Codex docs, upstream source references, and local `$CODEX_HOME` state. Fixed schema metadata, separated OS-specific config paths, distinguished interactive defaults from `codex exec` defaults, and retained the need for Claudine PolicyEngine updates.
- 2026-07-02: Prior research captured Codex sandbox/approval split, permission profiles, execpolicy rules, managed policy, MCP filters, protected paths, and subagent overrides.
